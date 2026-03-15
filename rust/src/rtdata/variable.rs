use std::collections::{HashMap, HashSet};

use log::{debug, warn};
use tokio::sync::broadcast;
use uuid::Uuid;
use prost::Message;

use super::parser::{parse_repeated_name, clone_name};
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
    AddBulkResponse,
    ListResponse,
    SetResponse,
    GetResponse,
    DelResponse,
    InvalidCmdResponse,
    OperationStatus,
    OptionalValue,
    NamespaceNode,
    value::Typed,
    command::CommandType,
    response::ResponseType,
    namespace_node::Node,
};

pub struct VariableManager {
    pub db: sled::Db,
    pub items_tree: sled::Tree,
    pub meta_tree: sled::Tree,
    pub root: String,
    pub tx: broadcast::Sender<VarIdValue>,
}

pub struct Subscriber {
    pub rx: broadcast::Receiver<VarIdValue>,
    pub sub_ids: Vec<String>,
}

impl Subscriber {
    pub async fn next_update(&mut self) -> Result<GetResponse, String> {
        loop {
            match self.rx.recv().await {
                Ok(var_id_val) => {
                    if let Some(index) = self.sub_ids.iter().position(|id| *id == var_id_val.var_id) {
                        let mut var_values = vec![OptionalValue { value: None }; self.sub_ids.len()];
                        var_values[index] = OptionalValue { value: var_id_val.value };
                        return Ok(GetResponse {
                            cmd_id: "".to_string(),
                            var_values,
                        });
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    return Err("Channel closed".to_string());
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    // Missed some messages, but we can keep trying
                    continue;
                }
            }
        }
    }
}

impl VariableManager {

    pub fn new(files_dir: &str) -> Self {
        let db = sled::open(files_dir).unwrap();
        let items_tree = db.open_tree("Variables").unwrap();
        let meta_tree = db.open_tree("Metadata").unwrap();

        let root_key = "root";
        let root_name = "root".to_string();

        let root_item: Option<Meta> = meta_tree.get(root_key).unwrap()
            .and_then(|bytes| Meta::decode(&*bytes).ok());

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

                items_tree.insert(uid.as_str(), root_folder.encode_to_vec()).unwrap();
                meta_tree.insert(root_key, app_meta.encode_to_vec()).unwrap();
                uid
            }
        };

        let (tx, _) = broadcast::channel(1024);
        Self { db, items_tree, meta_tree, root, tx }
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

    fn get_item(&self, id: &str) -> Result<Item, String> {
        match self.items_tree.get(id) {
            Ok(Some(bytes)) => {
                Item::decode(&*bytes).map_err(|e| format!("Error decoding item {id}: {e}"))
            }
            Ok(None) => Err(format!("Item with id: `{id}` not found.")),
            Err(e) => Err(format!("Error getting item with id: `{id}`: {e}")),
        }
    }

    fn add_items(&self, parent_id: &str, items_meta: Vec<ItemMeta>) -> Result<Vec<String>, String> {
        let base_err = format!("Error creating items.");
        let mut parent_item = self.get_item(parent_id)?;
        let mut do_put_parent = false;
        let mut item_ids = vec![];
        for i_meta in items_meta {
            match parent_item.children.get(&i_meta.name) {
                Some(child) => {
                    item_ids.push(child.child_id.clone());
                }
                None => {
                    let uid = Uuid::new_v4().to_string();
                    do_put_parent = true;
                    let item = Item {
                        id: uid.clone(),
                        name: i_meta.name,
                        parent: Some(parent_id.to_string()),
                        i_type: i_meta.i_type,
                        var_d_type: i_meta.var_d_type,
                        ..Default::default()
                    };
                    parent_item.children.insert(item.name.clone(), ChildInfo { child_id: item.id.clone(), i_type: item.i_type, var_d_type: item.var_d_type});
                    match self.items_tree.insert(&item.id, item.encode_to_vec()) {
                        Ok(_) => item_ids.push(uid),
                        Err(e) => return Err(format!("{base_err} Insert item error: {e}"))
                    };
                }
            }
        }
        if do_put_parent {
            match self.items_tree.insert(parent_id, parent_item.encode_to_vec()) {
                Ok(_) => (),
                Err(e) => return Err(format!("{base_err} Put item error: {e}"))
            };
        }
        Ok(item_ids)
    }

    fn add_bulk_recursive(&self, parent_id: &str, nodes: HashMap<String, NamespaceNode>, is_root: bool) -> Result<(), String> {
        for (key, val) in nodes {
            let mut name_ = key.as_str();
            let (start, stop, step) = parse_repeated_name(&mut name_);
            let item_names = clone_name(name_, start, stop, step);

            let mut vars_to_create: Vec<ItemMeta> = vec![];
            let mut folders_to_create: Vec<ItemMeta> = vec![];
            let mut folder_children_to_create: Vec<HashMap<String, NamespaceNode>> = vec![];

            for name in item_names {
                match &val.node {
                    Some(Node::Folder(folder)) => {
                        debug!("Bulk ADD: Creating folder '{}' under parent '{}'", name, parent_id);
                        let i_meta = ItemMeta {
                            name: name.clone(),
                            i_type: ItemType::Folder as i32,
                            var_d_type: None,
                            uid: None,
                        };
                        folders_to_create.push(i_meta);
                        folder_children_to_create.push(folder.children.clone());
                    }
                    Some(Node::VariableType(v_type)) => {
                        debug!("Bulk ADD: Creating variable '{}' (type: {}) under parent '{}'", name, v_type, parent_id);
                        let v_dtype = match v_type.to_lowercase().as_str() {
                            "integer" | "int" | "i" => VarDataType::Integer as i32,
                            "float" | "f" => VarDataType::Float as i32,
                            "text" | "string" | "str" | "t" => VarDataType::Text as i32,
                            "boolean" | "bool" | "b" => VarDataType::Boolean as i32,
                            _ => {
                                warn!("Bulk ADD: Invalid variable type '{}' for '{}'", v_type, name);
                                VarDataType::Invalid as i32
                            },
                        };
                        vars_to_create.push(ItemMeta {
                            name: name.clone(),
                            i_type: ItemType::Variable as i32,
                            var_d_type: Some(v_dtype),
                            uid: None,
                        });
                    }
                    None => {
                        warn!("Bulk ADD: Node with key '{}' has no content defined", name);
                    },
                }
            }
            self.add_items(parent_id, vars_to_create)?;
            let created_folder_ids = self.add_items(parent_id, folders_to_create)?;

            for (folder_id, children) in created_folder_ids.iter().zip(folder_children_to_create.iter()) {
                if is_root {
                    debug!("Creating children for: {folder_id}");
                }
                self.add_bulk_recursive(folder_id, children.clone(), false)?;
            }
        }
        Ok(())
    }

    fn set_vals(&self, var_id_vals: Vec<VarIdValue>) -> Result<(bool, Vec<VarIdValue>), String> {
        let base_err = format!("Error setting variable values.");
        let mut do_commit = false;
        let mut successfully_set_vals = vec![];
        for var_id_val in var_id_vals {
            let var_id = var_id_val.var_id.clone();
            match (self.items_tree.get(&var_id), var_id_val.value) {
                (Ok(Some(bytes)), Some(value)) => {
                    let mut var = Item::decode(&*bytes).map_err(|e| format!("{base_err} Decode: {e}"))?;
                    let i_type = VariableManager::cast_item_type(var.i_type);
                    let v_dtype = VariableManager::cast_var_data_type(var.var_d_type);
                    let val = value.typed.ok_or_else(|| format!("None value"))?;
                    match (i_type, v_dtype, val) {
                        (ItemType::Variable, VarDataType::Integer, Typed::IntegerValue(v)) => var.value = Some(Value {typed: Some(Typed::IntegerValue(v))}),
                        (ItemType::Variable, VarDataType::Integer, Typed::FloatValue(v)) => var.value = Some(Value {typed: Some(Typed::IntegerValue(v as i64))}),
                        (ItemType::Variable, VarDataType::Float, Typed::FloatValue(v)) => var.value = Some(Value {typed: Some(Typed::FloatValue(v))}),
                        (ItemType::Variable, VarDataType::Float, Typed::IntegerValue(v)) => var.value = Some(Value {typed: Some(Typed::FloatValue(v as f64))}),
                        (ItemType::Variable, VarDataType::Text, Typed::TextValue(v)) => var.value = Some(Value {typed: Some(Typed::TextValue(v))}),
                        (ItemType::Variable, VarDataType::Boolean, Typed::BooleanValue(v)) => var.value = Some(Value {typed: Some(Typed::BooleanValue(v))}),
                        (ItemType::Variable, VarDataType::Invalid, _) => return Err(format!("Invalid var data type.")),
                        (ItemType::Variable, _, _) => return Err(format!("Mismatch data type.")),
                        (ItemType::Folder, _, _) => return Err(format!("Can't write value to a folder.")),
                        (ItemType::Invalid, _, _) => return Err(format!("Invalid item type.")),
                    }
                    match self.items_tree.insert(&var_id, var.encode_to_vec()) {
                        Ok(_) => {
                            do_commit = true;
                            successfully_set_vals.push(VarIdValue {
                                var_id,
                                value: var.value.clone(),
                            });
                        },
                        Err(e) => return Err(format!("{base_err} Putting var: {e}"))
                    }
                }
                (Ok(Some(_)), None) => return Err(format!("{base_err} Can't set a null value")),
                (Ok(None), _) => return Err(format!("{base_err} Variable not found")),
                (Err(e), _) => return Err(format!("{base_err} Getting variable: {e}"))
            }
        }

        Ok((do_commit, successfully_set_vals))
    }

    fn get_vals(&self, var_ids: Vec<String>) -> Result<Vec<OptionalValue>, String> {
        let base_err = format!("Error getting variable values.");
        let mut values = vec![];
        for var_id in var_ids {
            match self.items_tree.get(&var_id) {
                Ok(Some(bytes)) => {
                    let var = Item::decode(&*bytes).map_err(|e| format!("{base_err} Decode: {e}"))?;
                    values.push(OptionalValue { value: var.value });
                }
                Ok(None) => return Err(format!("{base_err} Variable {var_id} not found")),
                Err(e) => return Err(format!("{base_err} Getting variable: {e}"))
            }
        }
        Ok(values)
    }

    fn get_all_descendants(&self, var_id: &String) -> Result<Vec<String>, String> {
        let mut stack = vec![var_id.clone()];
        let mut descendants = vec![];
        let base_err = format!("Error getting descendants.");
        loop {
            match stack.pop() {
                Some(curr_id) => {
                    match self.items_tree.get(&curr_id) {
                        Ok(Some(bytes)) => {
                            let var = Item::decode(&*bytes).map_err(|e| format!("{base_err} Decode: {e}"))?;
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

    fn del_items(&self, items_ids: Vec<String>) -> Result<bool, String> {
        let mut cascade_children_to_rm: HashSet<String> = HashSet::new();
        let mut do_commit: bool = false;
        let base_err = format!("Error deleting variables.");
        for id in items_ids {
            let desc = self.get_all_descendants(&id)?;
            cascade_children_to_rm.extend(desc);
            cascade_children_to_rm.insert(id.clone());
            let item = self.get_item(&id)?;
            match item.parent {
                Some(parent_id) => {
                    let mut parent = self.get_item(&parent_id)?;
                    parent.children.remove(&item.name);
                    match self.items_tree.insert(&parent_id, parent.encode_to_vec()) {
                        Ok(_) => do_commit = true,
                        Err(e) => return Err(format!("{base_err} Putting var: {e}"))
                    }
                }
                None => return Err(format!("Can't remove root variable"))
            }
        }
        for id_to_rm in cascade_children_to_rm {
            match self.items_tree.remove(&id_to_rm) {
                Ok(_) => do_commit = true,
                Err(e) => return Err(format!("{base_err} Removing var: {e}"))
            }
        }
        Ok(do_commit)
    }

    pub fn exec_cmd(&self, cmd: Command) -> Response {
        match cmd.command_type {
            Some(CommandType::Add(add_cmd)) => {
                let (status, error_msg, item_ids) = match self.add_items(&add_cmd.parent_id, add_cmd.items_meta) {
                    Ok(inserted_ids) => (OperationStatus::Ok as i32, None, inserted_ids),
                    Err(e) => (OperationStatus::Err as i32, Some(format!("Inserting error: {e}")), vec![])
                };
                Response { response_type: Some(ResponseType::Add(AddResponse { cmd_id: add_cmd.cmd_id, item_ids })), status, error_msg }
            }
            Some(CommandType::List(list_cmd)) => {
                let mut children_folders: HashMap<String, String> = HashMap::new();
                let mut children_vars: HashMap<String, VarInfo> = HashMap::new();
                let (status, error_msg) = match list_cmd.folder_id {
                    Some(folder_id) => {
                        match self.get_item(&folder_id) {
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
                    None => {
                        children_folders.insert("root".to_string(), self.root.clone());
                        (OperationStatus::Ok as i32, None)
                    }
                };
                Response { response_type: Some(ResponseType::List(ListResponse {
                    cmd_id: list_cmd.cmd_id,
                    children_folders,
                    children_vars
                })), status, error_msg }
            }
            Some(CommandType::Set(set_cmd)) => {
                let (status, error_msg) = match self.set_vals(set_cmd.var_ids_values) {
                    Ok((_do_commit, successfully_set_vals)) => {
                        for v in successfully_set_vals {
                            let _ = self.tx.send(v);
                        }
                        (OperationStatus::Ok as i32, None)
                    }
                    Err(e) => (OperationStatus::Err as i32, Some(e))
                };
                Response { response_type: Some(ResponseType::Set(SetResponse { cmd_id: set_cmd.cmd_id })), status, error_msg }
            }
            Some(CommandType::Get(get_cmd)) => {
                let (status, error_msg, var_values) = match self.get_vals(get_cmd.var_ids) {
                    Ok(vals) => (OperationStatus::Ok as i32, None, vals),
                    Err(e) => (OperationStatus::Err as i32, Some(e), vec![])
                };
                Response { response_type: Some(ResponseType::Get(GetResponse { cmd_id: get_cmd.cmd_id, var_values })), status, error_msg }
            }
            Some(CommandType::Del(del_cmd)) => {
                let (status, error_msg) = match self.del_items(del_cmd.item_ids) {
                    Ok(_do_commit) => (OperationStatus::Ok as i32, None),
                    Err(e) => (OperationStatus::Err as i32, Some(e))
                };
                Response { response_type: Some(ResponseType::Del(DelResponse { cmd_id: del_cmd.cmd_id })), status, error_msg }
            }
            Some(CommandType::AddBulk(addb_cmd)) => {
                let (status, error_msg) = match addb_cmd.schema {
                    Some(schema) => {
                        match self.add_bulk_recursive(&addb_cmd.parent_id, schema.roots, true) {
                            Ok(_) => (OperationStatus::Ok as i32, None),
                            Err(e) => (OperationStatus::Err as i32, Some(format!("Bulk insert error: {e}")))
                        }
                    }
                    None => (OperationStatus::Err as i32, Some(format!("No schema provided")))
                };
                Response { response_type: Some(ResponseType::AddBulk(AddBulkResponse { cmd_id: addb_cmd.cmd_id })), status, error_msg }
            }
            None => {
                let uid = Uuid::new_v4().to_string();
                Response { response_type: Some(ResponseType::Inv(InvalidCmdResponse {
                    cmd_id: uid
                })),
                status: OperationStatus::Err as i32,
                error_msg: Some("No valid command received".to_string())
                }
            }
        }
    }

    pub fn subscribe(&self, var_ids: Vec<String>) -> Subscriber {
        Subscriber {
            rx: self.tx.subscribe(),
            sub_ids: var_ids
        }
    }
}
