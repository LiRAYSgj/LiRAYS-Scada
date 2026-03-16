use std::collections::{HashMap, HashSet};

use log::{debug, warn, info};
use tokio::sync::broadcast;
use uuid::Uuid;
use prost::Message;
use sled::{Db, Tree, Batch};

use super::parser::{parse_repeated_name, clone_name};
use super::namespace::{
    Item,
    ItemType,
    ItemMeta,
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
    pub db: Db,
    pub items_tree: Tree,
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
        let items_tree = db.open_tree("mainTree").unwrap();

        let (tx, _) = broadcast::channel(1024);
        Self { db, items_tree, tx }
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

    fn normalize_path(path: &str, i_type: ItemType) -> String {
        let trimmed = path.trim_matches('/');
        let components: Vec<&str> = trimmed.split('/').filter(|s| !s.is_empty()).collect();
        let mut base = format!("/{}", components.join("/"));
        match i_type {
            ItemType::Folder => base.push('/'),
            _ => ()
        }
        base
    }

    fn get_ancestors(path: &str) -> Vec<(String, String)> {
        let mut ancestors = vec![];
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        let mut parent = format!("/");
        for part in parts {
            ancestors.push((parent.clone(), part.to_string()));
            parent.push_str(format!("{}/", part).as_str());
        }
        ancestors
    }

    fn get_hierarchy_key(full_path: &str) -> String {
        let normalized = VariableManager::normalize_path(full_path, ItemType::Variable);
        let (parent, name) = normalized.rsplit_once('/').unwrap_or(("", &normalized));
        format!("H:{}/\0{}", parent, name)
    }

    fn add_items(&self, parent_id: &str, items_meta: Vec<ItemMeta>, batch: &mut Batch) -> Result<usize, String> {
        // Let's verify parent_id is an existing folder. Or create it otherwise
        let parent_path = VariableManager::normalize_path(parent_id, ItemType::Folder);
        let ancestors = VariableManager::get_ancestors(&parent_path);
        let mut count = 0;

        for (parent, folder_name) in ancestors {
            let h_key = format!("H:{}\0{}", parent, folder_name);
            match self.items_tree.get(&h_key).map_err(|e| format!("Reading error: {e}"))? {
                None => {
                    let item = ItemMeta {
                        name: folder_name,
                        i_type: ItemType::Folder as i32,
                        var_d_type: None,
                    };
                    batch.insert(h_key.as_bytes(), item.encode_to_vec());
                    count += 1;
                }
                _ => ()
            }
        }

        for i_meta in items_meta {
            let h_key = format!("H:{}\0{}", parent_path, i_meta.name);
            batch.insert(h_key.as_bytes(), i_meta.encode_to_vec());
            count += 1;
        }
        Ok(count)
    }

    fn list_path(&self, parent_id: &str) -> Result<(Vec<ItemMeta>, Vec<ItemMeta>), String> {
        let path = VariableManager::normalize_path(parent_id, ItemType::Folder);
        let prefix = format!("H:{}\0", path);
        let mut children_folders = vec![];
        let mut children_vars = vec![];

        for result in self.items_tree.scan_prefix(prefix) {
            let (_, _value) = result.map_err(|e| format!("Error reading tree: {e}"))?;
            let i_meta = ItemMeta::decode(_value.as_ref()).map_err(|e| format!("Error decoding Item: {e}"))?;
            let i_type = VariableManager::cast_item_type(i_meta.i_type);
            match i_type {
                ItemType::Folder => children_folders.push(i_meta),
                ItemType::Variable => children_vars.push(i_meta),
                _ => return Err(format!("Invalid item type"))
            }
        }

        Ok((children_folders, children_vars))
    }

    fn add_bulk_recursive(&self, root_parent_id: &str, root_nodes: HashMap<String, NamespaceNode>, batch: &mut Batch) -> Result<(), String> {
        let mut stack = vec![(root_parent_id.to_string(), root_nodes)];
        let mut total_count = 0usize;

        while let Some((parent_id, nodes)) = stack.pop() {
            for (key, val) in nodes {
                let mut name_ = key.as_str();
                let (start, stop, step) = parse_repeated_name(&mut name_);
                let item_names = clone_name(name_, start, stop, step);

                let mut vars_to_create: Vec<ItemMeta> = vec![];
                let mut folders_to_create: Vec<ItemMeta> = vec![];
                let mut folder_children: Vec<(String, HashMap<String, NamespaceNode>)> = vec![];

                for name in item_names {
                    match &val.node {
                        Some(Node::Folder(folder)) => {
                            folders_to_create.push(ItemMeta {
                                name: name.clone(),
                                i_type: ItemType::Folder as i32,
                                var_d_type: None,
                            });
                            let new_parent_path = if parent_id == "/" {
                                format!("/{}", name)
                            } else {
                                format!("{}/{}", parent_id.trim_end_matches('/'), name)
                            };
                            folder_children.push((new_parent_path, folder.children.clone()));
                        }
                        Some(Node::VariableType(v_type)) => {
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
                            });
                        }
                        None => {
                            warn!("Bulk ADD: Node with key '{}' has no content defined", name);
                        }
                    }
                }

                if !vars_to_create.is_empty() {
                    let prev_m = total_count / 1_000_000;
                    total_count += self.add_items(&parent_id, vars_to_create, batch)?;
                    let curr_m = total_count / 1_000_000;
                    if curr_m > prev_m {
                        info!("Bulk ADD: {} million items added to batch", curr_m);
                    }
                }
                if !folders_to_create.is_empty() {
                    let prev_m = total_count / 1_000_000;
                    total_count += self.add_items(&parent_id, folders_to_create, batch)?;
                    let curr_m = total_count / 1_000_000;
                    if curr_m > prev_m {
                        debug!("Bulk ADD: {} million items added to batch", curr_m);
                    }
                }

                for (child_path, children) in folder_children {
                    stack.push((child_path, children));
                }
            }
        }
        Ok(())
    }

    fn set_vals(&self, var_id_vals: Vec<VarIdValue>) -> Result<((), Vec<VarIdValue>), String> {
        let base_err = format!("Error setting variable values.");
        let mut batch = Batch::default();
        let mut successfully_set_vals = vec![];
        for var_id_val in var_id_vals {
            let var_id = VariableManager::normalize_path(&var_id_val.var_id, ItemType::Variable);
            let h_key = VariableManager::get_hierarchy_key(&var_id_val.var_id);
            let i_meta = match self.items_tree.get(h_key.as_bytes()) {
                Ok(Some(_value)) => ItemMeta::decode(_value.as_ref()).map_err(|e| format!("Error decoding Item: {e}"))?,
                Ok(None) => return Err(format!("Missing variable.")),
                Err(e) => return Err(format!("Error reading tree: {e}"))
            };
            let d_key_str = format!("D:{}", var_id);
            let d_key = d_key_str.as_bytes();
            let i_type = VariableManager::cast_item_type(i_meta.i_type);
            let v_dtype = VariableManager::cast_var_data_type(i_meta.var_d_type);
            match var_id_val.value {
                Some(value) => {
                    let val = value.typed.ok_or_else(|| format!("None value"))?;
                    match (i_type, v_dtype, val) {
                        (ItemType::Variable, VarDataType::Integer, Typed::IntegerValue(v)) => batch.insert(d_key, Value {typed: Some(Typed::IntegerValue(v))}.encode_to_vec()),
                        (ItemType::Variable, VarDataType::Integer, Typed::FloatValue(v)) => batch.insert(d_key, Value {typed: Some(Typed::IntegerValue(v as i64))}.encode_to_vec()),
                        (ItemType::Variable, VarDataType::Float, Typed::FloatValue(v)) => batch.insert(d_key, Value {typed: Some(Typed::FloatValue(v))}.encode_to_vec()),
                        (ItemType::Variable, VarDataType::Float, Typed::IntegerValue(v)) => batch.insert(d_key, Value {typed: Some(Typed::FloatValue(v as f64))}.encode_to_vec()),
                        (ItemType::Variable, VarDataType::Text, Typed::TextValue(v)) => batch.insert(d_key, Value {typed: Some(Typed::TextValue(v))}.encode_to_vec()),
                        (ItemType::Variable, VarDataType::Boolean, Typed::BooleanValue(v)) => batch.insert(d_key, Value {typed: Some(Typed::BooleanValue(v))}.encode_to_vec()),
                        (ItemType::Variable, VarDataType::Invalid, _) => return Err(format!("Invalid var data type.")),
                        (ItemType::Variable, _, _) => return Err(format!("Mismatch data type.")),
                        (ItemType::Folder, _, _) => return Err(format!("Can't write value to a folder.")),
                        (ItemType::Invalid, _, _) => return Err(format!("Invalid item type.")),
                    }
                }
                None => return Err(format!("{base_err} Can't set a null value"))
            }
        }
        self.items_tree.apply_batch(batch).map_err(|e| format!("Error Applying batch op: {e}"))?;
        Ok(((), successfully_set_vals))
    }

    fn get_vals(&self, var_ids: Vec<String>) -> Result<Vec<OptionalValue>, String> {
        let base_err = format!("Error getting variable values.");
        let mut values = vec![];
        for var_id in var_ids {
            let d_key_str = format!("D:{}", var_id);
            match self.items_tree.get(&d_key_str) {
                Ok(Some(_bytes)) => {
                    let value = Value::decode(_bytes.as_ref()).map_err(|e| format!("Error decoding Value: {e}"))?;
                    values.push(OptionalValue { value: Some(value) });
                }
                Ok(None) => { values.push(OptionalValue { value: None }); },
                Err(e) => return Err(format!("{base_err} Getting variable: {e}"))
            }
        }
        Ok(values)
    }

    fn del_items(&self, items_ids: Vec<String>) -> Result<(), String> {
        let mut batch = Batch::default();
        for id in items_ids {
            let as_var_id = VariableManager::normalize_path(&id, ItemType::Variable);
            let potential_d_key_str = format!("D:{}", as_var_id);
            let potential_d_key = potential_d_key_str.as_bytes();
            let potential_ref_str = VariableManager::get_hierarchy_key(&id);
            let potential_ref = potential_ref_str.as_bytes();

            batch.remove(potential_d_key);  // Remove value if it is a var and has a value
            batch.remove(potential_ref);  // Remove current reference

            // Remove All potential children references
            let prefix = format!("H:{}/", as_var_id);
            for result in self.items_tree.scan_prefix(prefix) {
                let (key, _) = result.map_err(|e| format!("Error reading tree: {e}"))?;
                batch.remove(key);
            }
            // Remove All potential children values
            let prefix = format!("D:{}/", as_var_id);
            for result in self.items_tree.scan_prefix(prefix) {
                let (key, _) = result.map_err(|e| format!("Error reading tree: {e}"))?;
                batch.remove(key);
            }
        }
        self.items_tree.apply_batch(batch).map_err(|e| format!("Error removing items: {e}"))
    }

    pub fn exec_cmd(&self, cmd: Command) -> Response {
        match cmd.command_type {
            Some(CommandType::Add(add_cmd)) => {
                let parent_id = add_cmd.parent_id.unwrap_or("/".to_string());
                let mut batch = Batch::default();
                let (status, error_msg) = match self.add_items(&parent_id, add_cmd.items_meta, &mut batch) {
                    Ok(_) => {
                        match self.items_tree.apply_batch(batch).map_err(|e| format!("Error adding items: {e}")) {
                            Ok(_) => (OperationStatus::Ok as i32, None),
                            Err(e) => (OperationStatus::Err as i32, Some(format!("Applying batch error: {e}")))
                        }
                    }
                    Err(e) => (OperationStatus::Err as i32, Some(format!("Inserting error: {e}")))
                };
                Response { response_type: Some(ResponseType::Add(AddResponse { cmd_id: add_cmd.cmd_id })), status, error_msg }
            }
            Some(CommandType::List(list_cmd)) => {
                let mut children_folders: HashMap<String, String> = HashMap::new();
                let mut children_vars: HashMap<String, VarInfo> = HashMap::new();
                let folder_to_list = list_cmd.folder_id.unwrap_or("/".to_string());
                let (status, error_msg) = match self.list_path(&folder_to_list) {
                    Ok((ch_folders, ch_vars)) => {
                        for ch_folder in ch_folders {
                            let var_id = format!("{}/{}", folder_to_list, ch_folder.name);
                            let ch_id = VariableManager::normalize_path(&var_id, ItemType::Variable);
                            children_folders.insert(ch_folder.name, ch_id);
                        }
                        for ch_var in ch_vars {
                            let var_id = format!("{}/{}", folder_to_list, ch_var.name);
                            let v_dtype = VariableManager::cast_var_data_type(ch_var.var_d_type);
                            children_vars.insert(ch_var.name, VarInfo { var_id, var_d_type: v_dtype as i32 });
                        }
                        (OperationStatus::Ok as i32, None)
                    }
                    Err(e) => (OperationStatus::Err as i32, Some(e))
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
                        let mut batch = sled::Batch::default();
                        match self.add_bulk_recursive(&addb_cmd.parent_id, schema.roots, &mut batch) {
                            Ok(_) => match self.items_tree.apply_batch(batch) {
                                Ok(_) => (OperationStatus::Ok as i32, None),
                                Err(e) => (OperationStatus::Err as i32, Some(format!("On apply batch: {e}"))),
                            },
                            Err(e) => (OperationStatus::Err as i32, Some(format!("Bulk insert error: {e}"))),
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
