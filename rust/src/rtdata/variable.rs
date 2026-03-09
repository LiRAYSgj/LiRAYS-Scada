use std::collections::{HashMap, HashSet};

use log::warn;
use uuid::Uuid;
use heed::{Database, Env, EnvOpenOptions, RoTxn, RwTxn, types::{SerdeJson, Str}};
use crate::rtdata::namespace::{
    AddResponse, Command, DelResponse, GetResponse, ListResponse, Response, SetResponse, Value, Variable
};

pub struct VariableManager {
    pub env: Env,
    pub vars_db: Database<Str, SerdeJson<Variable>>,
    pub root: Option<String>,
}

impl VariableManager {

    pub fn new(files_dir: &str) -> Self {
        let env = unsafe { EnvOpenOptions::new().max_dbs(2).open(files_dir).unwrap() };
        let mut rw_txn = env.write_txn().unwrap();
        let vars_db = env.create_database(&mut rw_txn, Some("Variables")).unwrap();
        rw_txn.commit().unwrap();
        let mut instance = Self { env, vars_db, root: None };
        instance.initialize_root();
        instance
    }

    fn initialize_root(&mut self) {
        let ro_txn = self.env.read_txn().unwrap();
        if self.vars_db.is_empty(&ro_txn).unwrap() {
            let uid_string = Uuid::new_v4().to_string();
            let uid_ = uid_string.as_str();
            let mut rw_txn = self.env.write_txn().unwrap();
            self.vars_db.put(&mut rw_txn, uid_, &Variable { name: "root".to_string(), ..Default::default()}).unwrap();
            rw_txn.commit().unwrap();
            self.root = Some(uid_string);
        }
    }

    fn get_var(&self, id: &str, ro_txn: &RoTxn) -> Result<Variable, String> {
        match self.vars_db.get(&ro_txn, id) {
            Ok(op_val) => {
                match op_val {
                    Some(val) => Ok(val),
                    None => Err(format!("Variable with id: `{id}` not found.")),
                }
            }
            Err(e) => {
                Err(format!("Error getting variable with id: `{id}`: {e}"))
            }
        }
    }

    fn add_vars(&self, parent_id: &str, names: Vec<String>, rw_txn: &mut RwTxn) -> Result<Vec<String>, String> {
        let base_err = format!("Error creating variables.");
        let mut ids = vec![];
        let mut parent_var = self.get_var(parent_id, rw_txn)?;
        let mut do_put_parent = false;
        for name in names {
            match parent_var.children_vars.get_mut(&name) {
                Some(id) => {
                    ids.push(self.get_var(&id, rw_txn)?.id);
                }
                None => {
                    let uid_string = Uuid::new_v4().to_string();
                    do_put_parent = true;
                    let var = Variable { id: uid_string, name: name.clone(), parent: Some(parent_id.to_string()), ..Default::default()};
                    parent_var.children_vars.insert(var.name.clone(), var.id.clone());
                    ids.push(var.id.clone());
                    match self.vars_db.put(rw_txn, &var.id, &var) {
                        Ok(()) => (),
                        Err(e) => return Err(format!("{base_err} Put parent error: {e}"))
                    };
                }
            }
        }
        if do_put_parent {
            match self.vars_db.put(rw_txn, &parent_id, &parent_var) {
                Ok(()) => (),
                Err(e) => return Err(format!("{base_err} Put parent error: {e}"))
            };
        }
        Ok(ids)
    }

    fn set_vals(&self, var_id_vals: Vec<(String, Value)>, rw_txn: &mut RwTxn) -> Result<bool, String> {
        let base_err = format!("Error setting variable values.");
        let mut do_commit = false;
        for (var_id, value) in var_id_vals {
            match self.vars_db.get(&rw_txn, &var_id) {
                Ok(Some(mut var)) => {
                    match value {
                        Value::Integer(v) => {
                            var.int_v = Some(v);
                            var.float_v = None;
                            var.txt_v = None;
                            var.bool_v = None;
                        }
                        Value::Float(v) => {
                            var.float_v = Some(v);
                            var.int_v = None;
                            var.txt_v = None;
                            var.bool_v = None;
                        }
                        Value::Text(v) => {
                            var.txt_v = Some(v);
                            var.int_v = None;
                            var.float_v = None;
                            var.bool_v = None;
                        }
                        Value::Boolean(v) => {
                            var.bool_v = Some(v);
                            var.int_v = None;
                            var.float_v = None;
                            var.txt_v = None;
                        }
                    }
                    match self.vars_db.put(rw_txn, &var_id, &var) {
                        Ok(_) => do_commit = true,
                        Err(e) => return Err(format!("{base_err} Putting var: {e}"))
                    }
                }
                Ok(None) => return Err(format!("{base_err} Variable {var_id} not found")),
                Err(e) => return Err(format!("{base_err} Getting variable: {e}"))
            }
        }
        Ok(do_commit)
    }

    fn get_vals(&self, var_ids: Vec<String>, ro_txn: &RoTxn) -> Result<Vec<Option<Value>>, String> {
        let base_err = format!("Error getting variable values.");
        let mut values = vec![];
        for var_id in var_ids {
            match self.vars_db.get(&ro_txn, &var_id) {
                Ok(Some(var)) => {
                    let val = match (var.int_v, var.float_v, var.txt_v, var.bool_v) {
                        (Some(v), None, None, None) => Some(Value::Integer(v)),
                        (None, Some(v), None, None) => Some(Value::Float(v)),
                        (None, None, Some(v), None) => Some(Value::Text(v)),
                        (None, None, None, Some(v)) => Some(Value::Boolean(v)),
                        _ => None
                    };
                    values.push(val);
                }
                Ok(None) => return Err(format!("{base_err} Variable {var_id} not found")),
                Err(e) => return Err(format!("{base_err} Getting variable: {e}"))
            }
        }
        Ok(values)
    }

    fn get_all_descendants(&self, var_id: &String, ro_txn: &RoTxn) -> Result<Vec<String>, String> {
        let mut stack = vec![var_id.clone()];
        let mut descendants = vec![];
        let base_err = format!("Error getting descendants.");
        loop {
            match stack.pop() {
                Some(curr_id) => {
                    match self.vars_db.get(&ro_txn, &curr_id) {
                        Ok(Some(var)) => {
                            let child_ids = var.children_vars.values().map(|x| x.to_owned()).collect::<Vec<String>>();
                            descendants.extend(child_ids.clone());
                            stack.extend(child_ids);
                        }
                        Ok(None) => warn!("Invalid data."),
                        Err(e) => {
                            return Err(format!("{base_err} Error getting var: {e}"))
                        }
                    }
                }
                None => break
            }
        }
        Ok(descendants)
    }

    fn del_vars(&self, var_ids: Vec<String>, rw_txn: &mut RwTxn) -> Result<bool, String> {
        let mut cascade_children_to_rm: HashSet<String> = HashSet::new();
        let mut do_commit: bool = false;
        let base_err = format!("Error deleting variables.");
        for id in var_ids {
            let desc = self.get_all_descendants(&id, rw_txn)?;
            cascade_children_to_rm.extend(desc);
            cascade_children_to_rm.insert(id.clone());
            let var = self.get_var(&id, rw_txn)?;
            match var.parent {
                Some(parent_id) => {
                    let mut parent = self.get_var(&parent_id, rw_txn)?;
                    parent.children_vars.remove(&var.name);
                    match self.vars_db.put(rw_txn, &parent_id, &parent) {
                        Ok(_) => do_commit = true,
                        Err(e) => return Err(format!("{base_err} Putting var: {e}"))
                    }
                }
                None => return Err(format!("Can't remove root variable"))
            }
        }
        for id_to_rm in cascade_children_to_rm {
            match self.vars_db.delete(rw_txn, &id_to_rm) {
                Ok(_) => do_commit = true,
                Err(e) => return Err(format!("{base_err} Removing var: {e}"))
            }
        }
        Ok(do_commit)
    }

    pub fn exec_cmd(&self, cmd: Command) -> Result<Response, String> {
        match cmd {
            Command::ADD(add_cmd) => {
                let mut rw_txn = self.env.write_txn().map_err(|e| format!("Creating transaction error: {e}"))?;
                let var_ids = self.add_vars(&add_cmd.parent_id, add_cmd.var_names, &mut rw_txn)?;
                if var_ids.len() > 0 {
                    rw_txn.commit().map_err(|e| format!("On commit: {e}"))?;
                }
                Ok(Response::ADD(AddResponse { cmd_id: add_cmd.cmd_id, var_ids }))
            }
            Command::LIST(list_cmd) => {
                match list_cmd.var_id {
                    Some(var_id) => {
                        let ro_txn = self.env.read_txn().map_err(|e| format!("Creating transaction error: {e}"))?;
                        let var = self.get_var(&var_id, &ro_txn)?;
                        Ok(Response::LIST(ListResponse { cmd_id: list_cmd.cmd_id, children: var.children_vars }))
                    }
                    None => {
                        match self.root.clone() {
                            Some(root) => {
                                let mut children = HashMap::new();
                                children.insert("root".to_string(), root);
                                Ok(Response::LIST(ListResponse { cmd_id: list_cmd.cmd_id, children }))
                            }
                            None => Err(format!("No root found. This error should not happen."))
                        }
                    }
                }
            }
            Command::SET(set_cmd) => {
                let mut rw_txn = self.env.write_txn().map_err(|e| format!("Creating transaction error: {e}"))?;
                let do_commit = self.set_vals(set_cmd.var_ids_values, &mut rw_txn)?;
                if do_commit {
                    rw_txn.commit().map_err(|e| format!("On commit: {e}"))?;
                }
                Ok(Response::SET(SetResponse { cmd_id: set_cmd.cmd_id }))
            }
            Command::GET(get_cmd) => {
                let ro_txn = self.env.read_txn().map_err(|e| format!("Creating transaction error: {e}"))?;
                let vals = self.get_vals(get_cmd.var_ids, &ro_txn)?;
                Ok(Response::GET(GetResponse { cmd_id: get_cmd.cmd_id, var_values: vals }))
            }
            Command::DEL(del_cmd) => {
                let mut rw_txn = self.env.write_txn().map_err(|e| format!("Creating transaction error: {e}"))?;
                let do_commit = self.del_vars(del_cmd.var_ids, &mut rw_txn)?;
                if do_commit {
                    rw_txn.commit().map_err(|e| format!("On commit: {e}"))?;
                }
                Ok(Response::DEL(DelResponse { cmd_id: del_cmd.cmd_id }))
            }
        }
    }
}
