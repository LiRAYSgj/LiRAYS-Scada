use std::collections::{HashMap, HashSet};

use log::warn;
use uuid::Uuid;
use heed::{Database, Env, EnvOpenOptions, RoTxn, RwTxn, types::{SerdeBincode, SerdeJson, Str}};
use crate::rtdata::namespace::VarDataType;

use super::namespace::{
    ItemType,
    Item,
    Value,
    AddResponse,
    DelResponse,
    GetResponse,
    ListResponse,
    Meta,
    Command,
    Response,
    SetResponse,
};


pub struct VariableManager {
    pub env: Env,
    pub items_db: Database<Str, SerdeJson<Item>>,
    pub meta_db: Database<Str, SerdeBincode<Meta>>,
    pub root: [u8; 16],
}

impl VariableManager {

    pub fn new(files_dir: &str) -> Self {
        let env = unsafe { EnvOpenOptions::new().max_dbs(2).open(files_dir).unwrap() };
        let mut rw_txn = env.write_txn().unwrap();
        let root_key = "root-id";
        let root_name = "root".to_string();
        let vars_db = env.create_database(&mut rw_txn, Some("Variables")).unwrap();
        let meta_db = env.create_database(&mut rw_txn, Some("Metadata")).unwrap();

        let root: [u8; 16] = match meta_db.get(&rw_txn, root_key).unwrap() {
            Some(Meta::RootUid(r_uid)) => r_uid,
            _ => {
                let uid_bytes = *Uuid::new_v4().as_bytes();
                let root_folder = Item {
                    id: Uuid::from_bytes(uid_bytes).to_string(),
                    name: root_name,
                    i_type: ItemType::Folder,
                    ..Default::default()
                };
                let uid = root_folder.id.clone();
                vars_db.put(&mut rw_txn, uid.as_str(), &root_folder).unwrap();
                meta_db.put(&mut rw_txn, root_key, &Meta::RootUid(uid_bytes)).unwrap();
                uid_bytes
            }
        };
        rw_txn.commit().unwrap();
        Self { env, items_db: vars_db, meta_db, root }
    }

    fn get_item(&self, id: &str, ro_txn: &RoTxn) -> Result<Item, String> {
        match self.items_db.get(&ro_txn, id) {
            Ok(op_val) => {
                match op_val {
                    Some(val) => Ok(val),
                    None => Err(format!("Item with id: `{id}` not found.")),
                }
            }
            Err(e) => {
                Err(format!("Error getting item with id: `{id}`: {e}"))
            }
        }
    }

    fn add_items(&self, parent_id: &str, items_meta: Vec<(String, ItemType, Option<VarDataType>)>, rw_txn: &mut RwTxn) -> Result<Vec<String>, String> {
        let base_err = format!("Error creating items.");
        let mut ids = vec![];
        let mut parent_item = self.get_item(parent_id, rw_txn)?;
        let mut do_put_parent = false;
        for (name, i_type, v_dtype) in items_meta {
            match parent_item.children.get_mut(&name) {
                Some((id, _, _)) => {
                    ids.push(self.get_item(&id, rw_txn)?.id);
                }
                None => {
                    let uid_string = Uuid::new_v4().to_string();
                    do_put_parent = true;
                    let item = Item {
                        id: uid_string,
                        name: name.clone(),
                        parent: Some(parent_id.to_string()),
                        i_type: i_type,
                        var_d_type: v_dtype,
                        ..Default::default()
                    };
                    parent_item.children.insert(item.name.clone(), (item.id.clone(), item.i_type.clone(), item.var_d_type.clone()));
                    ids.push(item.id.clone());
                    match self.items_db.put(rw_txn, &item.id, &item) {
                        Ok(()) => (),
                        Err(e) => return Err(format!("{base_err} Put parent error: {e}"))
                    };
                }
            }
        }
        if do_put_parent {
            match self.items_db.put(rw_txn, &parent_id, &parent_item) {
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
            match self.items_db.get(&rw_txn, &var_id) {
                Ok(Some(mut var)) => {
                    match (var.i_type.clone(), var.var_d_type.clone(), value) {
                        (ItemType::Variable, Some(VarDataType::Integer), Value::Integer(v)) => var.value = Some(Value::Integer(v)),
                        (ItemType::Variable, Some(VarDataType::Float), Value::Float(v)) => var.value = Some(Value::Float(v)),
                        (ItemType::Variable, Some(VarDataType::Text), Value::Text(v)) => var.value = Some(Value::Text(v)),
                        (ItemType::Variable, Some(VarDataType::Boolean), Value::Boolean(v)) => var.value = Some(Value::Boolean(v)),
                        (ItemType::Variable, _, _) => return Err(format!("Mismatch on var data type and value: {var_id}")),
                        (ItemType::Folder, _, _) => return Err(format!("Can't set value to an item folder"))
                    }
                    match self.items_db.put(rw_txn, &var_id, &var) {
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
            match self.items_db.get(&ro_txn, &var_id) {
                Ok(Some(var)) => {
                    values.push(var.value);
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
                    match self.items_db.get(&ro_txn, &curr_id) {
                        Ok(Some(var)) => {
                            for (id_, i_type, _) in var.children.values() {
                                descendants.push(id_.clone());
                                match i_type {
                                    ItemType::Folder => stack.push(id_.to_owned()),
                                    _ => ()
                                }
                            }
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

    fn del_items(&self, items_ids: Vec<String>, rw_txn: &mut RwTxn) -> Result<bool, String> {
        let mut cascade_children_to_rm: HashSet<String> = HashSet::new();
        let mut do_commit: bool = false;
        let base_err = format!("Error deleting variables.");
        for id in items_ids {
            let desc = self.get_all_descendants(&id, rw_txn)?;
            cascade_children_to_rm.extend(desc);
            cascade_children_to_rm.insert(id.clone());
            let item = self.get_item(&id, rw_txn)?;
            match item.parent {
                Some(parent_id) => {
                    let mut parent = self.get_item(&parent_id, rw_txn)?;
                    parent.children.remove(&item.name);
                    match self.items_db.put(rw_txn, &parent_id, &parent) {
                        Ok(_) => do_commit = true,
                        Err(e) => return Err(format!("{base_err} Putting var: {e}"))
                    }
                }
                None => return Err(format!("Can't remove root variable"))
            }
        }
        for id_to_rm in cascade_children_to_rm {
            match self.items_db.delete(rw_txn, &id_to_rm) {
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
                let item_ids = self.add_items(&add_cmd.parent_id, add_cmd.items_meta, &mut rw_txn)?;
                if item_ids.len() > 0 {
                    rw_txn.commit().map_err(|e| format!("On commit: {e}"))?;
                }
                Ok(Response::ADD(AddResponse { cmd_id: add_cmd.cmd_id, item_ids }))
            }
            Command::LIST(list_cmd) => {
                match list_cmd.item_id {
                    Some(folder_id) => {
                        let ro_txn = self.env.read_txn().map_err(|e| format!("Creating transaction error: {e}"))?;
                        let var = self.get_item(&folder_id, &ro_txn)?;
                        let mut children_folders = HashMap::new();
                        let mut children_vars = HashMap::new();
                        for (name, (item_id, i_type, v_type)) in var.children {
                            match (i_type, v_type) {
                                (ItemType::Folder, _) => {children_folders.insert(name, item_id);},
                                (ItemType::Variable, Some(d_t)) => {children_vars.insert(name, (item_id, d_t));},
                                (ItemType::Variable, None) => warn!("Invalid Data Found. Variables must have a data type"),
                            }
                        }
                        Ok(Response::LIST(ListResponse { cmd_id: list_cmd.cmd_id, children_folders, children_vars }))
                    }
                    None => {
                        let mut children_folders = HashMap::new();
                        let children_vars = HashMap::new();
                        let root_id = Uuid::from_bytes(self.root).to_string();
                        children_folders.insert("root".to_string(), root_id);
                        Ok(Response::LIST(ListResponse { cmd_id: list_cmd.cmd_id, children_folders, children_vars }))
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
                let do_commit = self.del_items(del_cmd.item_ids, &mut rw_txn)?;
                if do_commit {
                    rw_txn.commit().map_err(|e| format!("On commit: {e}"))?;
                }
                Ok(Response::DEL(DelResponse { cmd_id: del_cmd.cmd_id }))
            }
        }
    }
}
