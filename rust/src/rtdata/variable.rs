use std::collections::{HashMap, HashSet};

use log::warn;
use uuid::Uuid;
use heed::{Database, Env, EnvOpenOptions, RoTxn, RwTxn, types::Str};

use super::proto::Proto;
use super::namespace::{
    Item,
    ItemType,
    ItemMeta,
    Meta,
    Value,
    VarIdValue,
    VarDataType,
    Command,
    Response,
    ChildInfo,
    VarInfo,
    AddResponse,
    ListResponse,
    SetResponse,
    GetResponse,
    DelResponse,
    InvalidCmdResponse,
    OperationStatus,
    OptionalValue,
    value::Typed,
    command::CommandType,
    response::ResponseType,
};

pub struct VariableManager {
    pub env: Env,
    pub items_db: Database<Str, Proto<Item>>,
    pub meta_db: Database<Str, Proto<Meta>>,
    pub root: String,
}

impl VariableManager {

    pub fn new(files_dir: &str) -> Self {

        let env = unsafe { EnvOpenOptions::new().max_dbs(2).open(files_dir).unwrap() };
        let mut rw_txn = env.write_txn().unwrap();
        let root_key = "root-id";
        let root_name = "root".to_string();
        let vars_db = env.create_database(&mut rw_txn, Some("Variables")).unwrap();
        let meta_db = env.create_database(&mut rw_txn, Some("Metadata")).unwrap();

        let root_item: Option<Meta> = meta_db.get(&rw_txn, root_key).unwrap();

        let root: String = match root_item {
            Some(meta_) => meta_.root_uid,
            _ => {
                let uid = Uuid::new_v4().to_string();
                let root_folder = Item {
                    id: uid.clone(),
                    name: root_name,
                    i_type: ItemType::Folder as i32,
                    ..Default::default()
                };

                let app_meta = Meta {
                    root_uid: uid.clone(),
                    vendor: "LiRAYS LLC".to_string()
                };

                vars_db.put(&mut rw_txn, uid.as_str(), &root_folder).unwrap();
                meta_db.put(&mut rw_txn, root_key, &app_meta).unwrap();
                uid
            }
        };
        rw_txn.commit().unwrap();
        Self { env, items_db: vars_db, meta_db, root }
    }

    fn cast_item_type(value: i32) -> ItemType {
        match ItemType::try_from(value) {
            Ok(it) => it,
            Err(_) => ItemType::Invalid
        }
    }

    fn cast_var_data_type(value: Option<i32>) -> VarDataType {
        match value {
            Some(v) => {
                match VarDataType::try_from(v) {
                    Ok(dt) => dt,
                    Err(_) => VarDataType::Invalid
                }
            }
            None => VarDataType::Invalid
        }
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

    fn add_items(&self, parent_id: &str, items_meta: Vec<ItemMeta>, rw_txn: &mut RwTxn) -> Result<usize, String> {
        let base_err = format!("Error creating items.");
        let mut parent_item = self.get_item(parent_id, rw_txn)?;
        let mut do_put_parent = false;
        let mut count = 0;
        for i_meta in items_meta {
            if !parent_item.children.contains_key(&i_meta.name) {
                let uid = Uuid::new_v4().to_string();
                do_put_parent = true;
                let item = Item {
                    id: uid,
                    name: i_meta.name,
                    parent: Some(parent_id.to_string()),
                    i_type: i_meta.i_type,
                    var_d_type: i_meta.var_d_type,
                    ..Default::default()
                };
                parent_item.children.insert(item.name.clone(), ChildInfo { child_id: item.id.clone(), i_type: item.i_type, var_d_type: item.var_d_type});
                match self.items_db.put(rw_txn, &item.id, &item) {
                    Ok(()) => count += 1,
                    Err(e) => return Err(format!("{base_err} Put parent error: {e}"))
                };
            }
        }
        if do_put_parent {
            match self.items_db.put(rw_txn, &parent_id, &parent_item) {
                Ok(()) => (),
                Err(e) => return Err(format!("{base_err} Put parent error: {e}"))
            };
        }
        Ok(count)
    }

    fn set_vals(&self, var_id_vals: Vec<VarIdValue>, rw_txn: &mut RwTxn) -> Result<bool, String> {
        let base_err = format!("Error setting variable values.");
        let mut do_commit = false;
        for var_id_val in var_id_vals {
            match (self.items_db.get(&rw_txn, &var_id_val.var_id), var_id_val.value) {
                (Ok(Some(mut var)), Some(value)) => {
                    let i_type = VariableManager::cast_item_type(var.i_type);
                    let v_dtype = VariableManager::cast_var_data_type(var.var_d_type);
                    let val = value.typed.ok_or_else(|| format!("None value"))?;
                    match (i_type, v_dtype, val) {
                        (ItemType::Variable, VarDataType::Integer, Typed::IntegerValue(v)) => var.value = Some(Value {typed: Some(Typed::IntegerValue(v))}),
                        (ItemType::Variable, VarDataType::Float, Typed::FloatValue(v)) => var.value = Some(Value {typed: Some(Typed::FloatValue(v))}),
                        (ItemType::Variable, VarDataType::Text, Typed::TextValue(v)) => var.value = Some(Value {typed: Some(Typed::TextValue(v))}),
                        (ItemType::Variable, VarDataType::Boolean, Typed::BooleanValue(v)) => var.value = Some(Value {typed: Some(Typed::BooleanValue(v))}),
                        (ItemType::Variable, VarDataType::Invalid, _) => return Err(format!("Invalid var data type.")),
                        (ItemType::Variable, _, _) => return Err(format!("Mismatch data type.")),
                        (ItemType::Folder, _, _) => return Err(format!("Can't write value to a folder.")),
                        (ItemType::Invalid, _, _) => return Err(format!("Invalid item type.")),
                    }
                    match self.items_db.put(rw_txn, &var_id_val.var_id, &var) {
                        Ok(_) => do_commit = true,
                        Err(e) => return Err(format!("{base_err} Putting var: {e}"))
                    }
                }
                (Ok(Some(_)), None) => return Err(format!("{base_err} Can't set a null value")),
                (Ok(None), _) => return Err(format!("{base_err} Variable not found")),
                (Err(e), _) => return Err(format!("{base_err} Getting variable: {e}"))
            }
        }
        Ok(do_commit)
    }

    fn get_vals(&self, var_ids: Vec<String>, ro_txn: &RoTxn) -> Result<Vec<OptionalValue>, String> {
        let base_err = format!("Error getting variable values.");
        let mut values = vec![];
        for var_id in var_ids {
            match self.items_db.get(&ro_txn, &var_id) {
                Ok(Some(var)) => {
                    values.push(OptionalValue { value: var.value });
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
                            for child in var.children.values() {
                                descendants.push(child.child_id.clone());
                                let ch_i_type = ItemType::try_from(child.i_type).map_err(|e| format!("Casting Item Type: {e}"))?;
                                match ch_i_type {
                                    ItemType::Folder => stack.push(child.child_id.to_owned()),
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

    pub fn exec_cmd(&self, cmd: Command) -> Response {
        match cmd.command_type {
            Some(CommandType::Add(add_cmd)) => {
                let (status, error_msg) = match self.env.write_txn() {
                    Ok(mut rw_txn) => {
                        match self.add_items(&add_cmd.parent_id, add_cmd.items_meta, &mut rw_txn) {
                            Ok(inserted) => {
                                if inserted > 0 {
                                    match rw_txn.commit() {
                                        Ok(_) => (OperationStatus::Ok as i32, None),
                                        Err(e) => (OperationStatus::Err as i32, Some(format!("On commit: {e}")))
                                    }
                                } else {
                                    (OperationStatus::Ok as i32, None)
                                }
                            }
                            Err(e) => (OperationStatus::Err as i32, Some(format!("Inserting error: {e}")))
                        }
                    }
                    Err(e) => (OperationStatus::Err as i32, Some(format!("Creating transaction error: {e}")))
                };
                Response { response_type: Some(ResponseType::Add(AddResponse { cmd_id: add_cmd.cmd_id, status, error_msg })) }
            }
            Some(CommandType::List(list_cmd)) => {
                let mut children_folders: HashMap<String, String> = HashMap::new();
                let mut children_vars: HashMap<String, VarInfo> = HashMap::new();
                let (status, error_msg) = match list_cmd.folder_id {
                    Some(folder_id) => {
                        match self.env.read_txn() {
                            Ok(ro_txn) => {
                                match self.get_item(&folder_id, &ro_txn) {
                                    Ok(var) => {
                                        for (name, child) in var.children {
                                            let i_type = VariableManager::cast_item_type(child.i_type);
                                            let v_dtype = VariableManager::cast_var_data_type(child.var_d_type);
                                            match (i_type, v_dtype) {
                                                (ItemType::Variable, VarDataType::Invalid) => warn!("Invalid variable type"),
                                                (ItemType::Variable, _) => {
                                                    children_vars.insert(name, VarInfo {
                                                        var_id: child.child_id, var_d_type: v_dtype as i32
                                                    });
                                                }
                                                (ItemType::Folder, _) => {
                                                    children_folders.insert(name, child.child_id);
                                                }
                                                (ItemType::Invalid, _) => warn!("Invalid item type")
                                            };
                                        }
                                        (OperationStatus::Ok as i32, None)
                                    }
                                    Err(e) => (OperationStatus::Err as i32, Some(e))
                                }
                            }
                            Err(e) => (OperationStatus::Err as i32, Some(format!("Creating transaction error: {e}")))
                        }
                    }
                    None => {
                        children_folders.insert("root".to_string(), self.root.clone());
                        (OperationStatus::Ok as i32, None)
                    }
                };
                Response { response_type: Some(ResponseType::List(ListResponse {
                    cmd_id: list_cmd.cmd_id,
                    status: status,
                    children_folders,
                    children_vars,
                    error_msg
                })) }
            }
            Some(CommandType::Set(set_cmd)) => {
                let (status, error_msg) = match self.env.write_txn() {
                    Ok(mut rw_txn) => {
                        match self.set_vals(set_cmd.var_ids_values, &mut rw_txn) {
                            Ok(do_commit) => {
                                if do_commit {
                                    match rw_txn.commit() {
                                        Ok(_) => (OperationStatus::Ok as i32, None),
                                        Err(e) => (OperationStatus::Err as i32, Some(format!("On commit: {e}")))
                                    }
                                } else {
                                    (OperationStatus::Ok as i32, None)
                                }
                            }
                            Err(e) => (OperationStatus::Err as i32, Some(e))
                        }
                    }
                    Err(e) => (OperationStatus::Err as i32, Some(format!("Creating transaction error: {e}")))
                };
                Response { response_type: Some(ResponseType::Set(SetResponse { cmd_id: set_cmd.cmd_id, status, error_msg })) }
            }
            Some(CommandType::Get(get_cmd)) => {
                let (status, error_msg, var_values) = match self.env.read_txn() {
                    Ok(ro_txn) => {
                        match self.get_vals(get_cmd.var_ids, &ro_txn) {
                            Ok(vals) => {
                                (OperationStatus::Ok as i32, None, vals)
                            }
                            Err(e) => (OperationStatus::Err as i32, Some(e), vec![])
                        }
                    }
                    Err(e) => (OperationStatus::Err as i32, Some(format!("Creating transaction error: {e}")), vec![])
                };
                Response { response_type: Some(ResponseType::Get(GetResponse { cmd_id: get_cmd.cmd_id, status, var_values, error_msg })) }
            }
            Some(CommandType::Del(del_cmd)) => {
                let (status, error_msg) = match self.env.write_txn() {
                    Ok(mut rw_txn) => {
                        match self.del_items(del_cmd.item_ids, &mut rw_txn) {
                            Ok(do_commit) => {
                                if do_commit {
                                    match rw_txn.commit() {
                                        Ok(_) => (OperationStatus::Ok as i32, None),
                                        Err(e) => (OperationStatus::Err as i32, Some(format!("On commit: {e}")))
                                    }
                                } else {
                                    (OperationStatus::Ok as i32, None)
                                }
                            }
                            Err(e) => (OperationStatus::Err as i32, Some(e))
                        }
                    }
                    Err(e) => (OperationStatus::Err as i32, Some(format!("Creating transaction error: {e}")))
                };
                Response { response_type: Some(ResponseType::Del(DelResponse { cmd_id: del_cmd.cmd_id, status, error_msg })) }
            }
            None => {
                let uid = Uuid::new_v4().to_string();
                Response { response_type: Some(ResponseType::Inv(InvalidCmdResponse {
                    cmd_id: uid,
                    status: OperationStatus::Err as i32,
                    error_msg: Some("No valid command received".to_string())
                })) }
            }
        }
    }
}
