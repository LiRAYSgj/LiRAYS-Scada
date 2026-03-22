use std::collections::{HashMap, HashSet};

use log::{debug, warn, info};
use tokio::sync::broadcast;
use uuid::Uuid;
use prost::Message;
use sled::{Db, Tree, Batch};

// use crate::rtdata::namespace::{AddCommand, ListCommand, SetCommand, SubscribeCommand};

use super::parser::{parse_repeated_name, clone_name};
use super::events::{extract_add_event, extract_del_event};
use super::utils::{
    cast_item_type,
    cast_var_data_type,
    normalize_path,
    get_ancestors,
    get_hierarchy_key,
    // generate_json_examples
};
use super::namespace::{
    ItemType,
    ItemMeta,
    Value,
    VarIdValue,
    VarDataType,
    Command,
    Response,
    FolderInfo,
    VarInfo,
    AddResponse,
    AddBulkResponse,
    ListResponse,
    SetResponse,
    GetResponse,
    DelResponse,
    SubscribeResponse,
    UnsubscribeResponse,
    InvalidCmdResponse,
    OperationStatus,
    OptionalValue,
    NamespaceNode,
    EventType,
    Event,
    value::Typed,
    event::Ev,
    command::CommandType,
    response::ResponseType,
    namespace_node::Node,
};

pub struct VariableManager {
    pub db: Db,
    pub items_tree: Tree,
    pub events_tx: broadcast::Sender<Event>,
}

impl VariableManager {

    pub fn new(files_dir: &str) -> Self {
        let db = sled::open(files_dir).unwrap();
        let items_tree = db.open_tree("mainTree").unwrap();

        let (tx, _) = broadcast::channel(1024);
        // generate_json_examples();
        Self { db, items_tree, events_tx: tx }
    }

    fn add_items(
        &self,
        parent_id: &str,
        items_meta: Vec<ItemMeta>,
        batch: &mut Batch
    ) -> Result<(bool, Vec<ItemMeta>, Vec<ItemMeta>), String> {
        // Let's verify parent_id is an existing folder. Or create it otherwise
        // Returns (reload: bool, new folders: Vec, new variables: Vec)
        let parent_path = normalize_path(parent_id, ItemType::Folder);
        let ancestors = get_ancestors(&parent_path);
        let mut new_folders = vec![];
        let mut new_variables = vec![];

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
                }
                _ => ()
            }
        }
        let prefix = format!("H:{}\0", parent_path);
        let children_keys: HashSet<String> = HashSet::from_iter(self.list_keys_with_prefix(&prefix)?);

        for i_meta in items_meta {
            let h_key = format!("H:{}\0{}", parent_path, i_meta.name);
            if children_keys.contains(&h_key) {
                return Err(format!("Can't create existing item {}", h_key));
            }
            batch.insert(h_key.as_bytes(), i_meta.encode_to_vec());
            let i_type = cast_item_type(i_meta.i_type);
            match i_type {
                ItemType::Folder => new_folders.push(i_meta),
                ItemType::Variable => new_variables.push(i_meta),
                ItemType::Invalid => warn!("Invalid item type in create")
            }
        }
        Ok((false, new_folders, new_variables))
    }

    fn list_path(&self, parent_id: &str) -> Result<(Vec<FolderInfo>, Vec<VarInfo>), String> {
        let path = normalize_path(parent_id, ItemType::Folder);
        let prefix = format!("H:{}\0", path);
        let mut children_folders: Vec<FolderInfo> = vec![];
        let mut children_vars: Vec<VarInfo> = vec![];

        for result in self.items_tree.scan_prefix(prefix) {
            let (_, _value) = result.map_err(|e| format!("Error reading tree: {e}"))?;
            let i_meta = ItemMeta::decode(_value.as_ref()).map_err(|e| format!("Error decoding Item: {e}"))?;
            let i_type = cast_item_type(i_meta.i_type);
            let var_id_ = format!("{}/{}", path, i_meta.name);
            let ch_id = normalize_path(&var_id_, ItemType::Variable);
            match i_type {
                ItemType::Folder => {children_folders.push(FolderInfo {id: ch_id, name: i_meta.name});}
                ItemType::Variable => {
                    let v_dtype = i_meta.var_d_type.ok_or("Invalid variable data type".to_string())?;
                    children_vars.push(VarInfo { id: ch_id, name: i_meta.name, var_d_type: v_dtype });
                }
                _ => return Err(format!("Invalid item type"))
            }
        }

        Ok((children_folders, children_vars))
    }

    pub fn list_keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>, String> {
        let mut keys = Vec::new();
        for result in self.items_tree.scan_prefix(prefix) {
            let (key, _) = result.map_err(|e| format!("Error reading tree: {e}"))?;
            keys.push(String::from_utf8(key.to_vec()).map_err(|e| format!("Error decoding key: {e}"))?);
        }
        Ok(keys)
    }

    fn add_bulk_recursive(
        &self,
        root_parent_id: &str,
        root_nodes: HashMap<String, NamespaceNode>,
        batch: &mut Batch
    ) -> Result<(Vec<ItemMeta>, Vec<ItemMeta>), String> {
        let mut stack = vec![(root_parent_id.to_string(), root_nodes)];
        let mut total_count = 0usize;
        let mut first_level_folders = vec![];
        let mut first_level_variables = vec![];

        while let Some((parent_id, nodes)) = stack.pop() {
            let is_first_level = parent_id == root_parent_id;
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
                    let (_, _, new_vars) = self.add_items(&parent_id, vars_to_create, batch)?;
                    total_count += new_vars.len();
                    let curr_m = total_count / 1_000_000;
                    if curr_m > prev_m {
                        info!("Bulk ADD: {} million items added to batch", curr_m);
                    }
                    // If this is the first level, collect the created variables
                    if is_first_level {
                        first_level_variables.extend(new_vars);
                    }
                }
                if !folders_to_create.is_empty() {
                    let prev_m = total_count / 1_000_000;
                    let (_, new_folders, _) = self.add_items(&parent_id, folders_to_create, batch)?;
                    total_count += new_folders.len();
                    let curr_m = total_count / 1_000_000;
                    if curr_m > prev_m {
                        debug!("Bulk ADD: {} million items added to batch", curr_m);
                    }
                    // If this is the first level, collect the created folders
                    if is_first_level {
                        first_level_folders.extend(new_folders);
                    }
                }

                for (child_path, children) in folder_children {
                    stack.push((child_path, children));
                }
            }
        }
        Ok((first_level_folders, first_level_variables))
    }

    fn set_vals(&self, var_id_vals: Vec<VarIdValue>) -> Result<Vec<VarIdValue>, String> {
        let base_err = format!("Error setting variable values.");
        let mut batch = Batch::default();
        let mut successfully_set_vals: Vec<VarIdValue> = vec![];
        for var_id_val in var_id_vals {
            let var_id = normalize_path(&var_id_val.var_id, ItemType::Variable);
            let h_key = get_hierarchy_key(&var_id_val.var_id);
            let i_meta = match self.items_tree.get(h_key.as_bytes()) {
                Ok(Some(_value)) => ItemMeta::decode(_value.as_ref()).map_err(|e| format!("Error decoding Item: {e}"))?,
                Ok(None) => return Err(format!("Missing variable.")),
                Err(e) => return Err(format!("Error reading tree: {e}"))
            };
            let d_key_str = format!("D:{}", var_id);
            let d_key = d_key_str.as_bytes();
            let i_type = cast_item_type(i_meta.i_type);
            let v_dtype = cast_var_data_type(i_meta.var_d_type);
            match var_id_val.value {
                Some(value) => {
                    let val = value.typed.ok_or_else(|| format!("None value"))?;
                    let value_ = match (i_type, v_dtype, val) {
                        (ItemType::Variable, VarDataType::Integer, Typed::IntegerValue(v)) => Value {typed: Some(Typed::IntegerValue(v))},
                        (ItemType::Variable, VarDataType::Integer, Typed::FloatValue(v)) => Value {typed: Some(Typed::IntegerValue(v as i64))},
                        (ItemType::Variable, VarDataType::Float, Typed::FloatValue(v)) => Value {typed: Some(Typed::FloatValue(v))},
                        (ItemType::Variable, VarDataType::Float, Typed::IntegerValue(v)) => Value {typed: Some(Typed::FloatValue(v as f64))},
                        (ItemType::Variable, VarDataType::Text, Typed::TextValue(v)) => Value {typed: Some(Typed::TextValue(v))},
                        (ItemType::Variable, VarDataType::Boolean, Typed::BooleanValue(v)) => Value {typed: Some(Typed::BooleanValue(v))},
                        (ItemType::Variable, VarDataType::Invalid, _) => return Err(format!("Invalid var data type.")),
                        (ItemType::Variable, _, _) => return Err(format!("Mismatch data type.")),
                        (ItemType::Folder, _, _) => return Err(format!("Can't write value to a folder.")),
                        (ItemType::Invalid, _, _) => return Err(format!("Invalid item type.")),
                    };
                    batch.insert(d_key, value_.encode_to_vec());
                    successfully_set_vals.push(VarIdValue { var_id, value: Some(value_)});
                }
                None => return Err(format!("{base_err} Can't set a null value"))
            }
        }
        self.items_tree.apply_batch(batch).map_err(|e| format!("Error Applying batch op: {e}"))?;
        Ok(successfully_set_vals)
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
            let as_var_id = normalize_path(&id, ItemType::Variable);
            let potential_d_key_str = format!("D:{}", as_var_id);
            let potential_d_key = potential_d_key_str.as_bytes();
            let potential_ref_str = get_hierarchy_key(&id);
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

    pub fn exec_cmd(
        &self,
        cmd: Command,
        subscribed_set: &mut HashSet<String>,
        get_tree_changes: &mut bool,
    ) -> Response {
        match cmd.command_type {
            Some(CommandType::Add(add_cmd)) => {
                let cmd_id = add_cmd.cmd_id.clone();
                let parent_id = add_cmd.parent_id.clone().unwrap_or("/".to_string());
                let mut batch = Batch::default();
                let (status, error_msg) = match self.add_items(&parent_id, add_cmd.items_meta.clone(), &mut batch) {
                    Ok((reload, new_folders, new_variables)) => {
                        match self.items_tree.apply_batch(batch).map_err(|e| format!("Error adding items: {e}")) {
                            Ok(_) => {
                                match add_cmd.parent_id {
                                    Some(folder_id) => {
                                        match extract_add_event(&folder_id, reload, new_folders, new_variables) {
                                            Ok(event) => {
                                                match self.events_tx.send(event) {
                                                    Ok(_) => (),
                                                    Err(e) => warn!("Error sending event: {e}")
                                                }
                                            }
                                            Err(e) => warn!("Error extracting event: {e}")
                                        }
                                        (OperationStatus::Ok as i32, None)
                                    }
                                    None => (OperationStatus::Err as i32, Some(format!("Missing parent id")))
                                }
                            }
                            Err(e) => (OperationStatus::Err as i32, Some(format!("Applying batch error: {e}")))
                        }
                    }
                    Err(e) => (OperationStatus::Err as i32, Some(format!("Inserting error: {e}")))
                };
                Response { response_type: Some(ResponseType::Add(AddResponse { cmd_id })), status, error_msg }
            }
            Some(CommandType::List(list_cmd)) => {
                let folder_to_list = list_cmd.folder_id.unwrap_or("/".to_string());
                let (status, error_msg, folders, variables) = match self.list_path(&folder_to_list) {
                    Ok((children_folders, children_vars)) => {
                        (OperationStatus::Ok as i32, None, children_folders, children_vars)
                    }
                    Err(e) => (OperationStatus::Err as i32, Some(e), vec![], vec![])
                };
                Response { response_type: Some(ResponseType::List(ListResponse { cmd_id: list_cmd.cmd_id, folders, variables })), status, error_msg }
            }
            Some(CommandType::Set(set_cmd)) => {
                let (status, error_msg) = match self.set_vals(set_cmd.var_ids_values) {
                    Ok(successfully_set_vals) => {
                        for v in successfully_set_vals {
                            match self.events_tx.send(Event { ev: Some(Ev::VarValueEv(v)) }) {
                                Ok(_) => (),
                                Err(e) => warn!("Error sending event: {e}")
                            }
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
                let (status, error_msg) = match self.del_items(del_cmd.item_ids.clone()) {
                    Ok(_) => {
                        match extract_del_event(&del_cmd) {
                            Ok(event) => {
                                match self.events_tx.send(event) {
                                    Ok(_) => (),
                                    Err(e) => warn!("Error sending event: {e}")
                                }
                            }
                            Err(e) => warn!("Error extracting event: {e}")
                        }
                        (OperationStatus::Ok as i32, None)
                    }
                    Err(e) => (OperationStatus::Err as i32, Some(e))
                };
                Response { response_type: Some(ResponseType::Del(DelResponse { cmd_id: del_cmd.cmd_id })), status, error_msg }
            }
            Some(CommandType::AddBulk(addb_cmd)) => {
                let (status, error_msg) = match addb_cmd.schema {
                    Some(schema) => {
                        let mut batch = sled::Batch::default();
                        match self.add_bulk_recursive(&addb_cmd.parent_id, schema.roots, &mut batch) {
                            Ok((new_folders, new_variables)) => match self.items_tree.apply_batch(batch) {
                                Ok(_) => {
                                    match extract_add_event(&addb_cmd.parent_id, false, new_folders, new_variables) {
                                        Ok(event) => {
                                            match self.events_tx.send(event) {
                                                Ok(_) => (),
                                                Err(e) => warn!("Error sending event: {e}")
                                            }
                                        }
                                        Err(e) => warn!("Error extracting event: {e}")
                                    }
                                    (OperationStatus::Ok as i32, None)
                                }
                                Err(e) => (OperationStatus::Err as i32, Some(format!("On apply batch: {e}"))),
                            },
                            Err(e) => (OperationStatus::Err as i32, Some(format!("Bulk insert error: {e}"))),
                        }
                    }
                    None => (OperationStatus::Err as i32, Some(format!("No schema provided")))
                };
                Response { response_type: Some(ResponseType::AddBulk(AddBulkResponse { cmd_id: addb_cmd.cmd_id })), status, error_msg }
            }
            Some(CommandType::Sub(sub_cmd)) => {
                if sub_cmd.events.contains(&(EventType::VarValues as i32)) {
                    subscribed_set.extend(sub_cmd.var_ids);
                }
                if sub_cmd.events.contains(&(EventType::TreeChange as i32)) {
                    *get_tree_changes = true;
                }
                Response {
                    response_type: Some(ResponseType::Sub(SubscribeResponse { cmd_id: sub_cmd.cmd_id })),
                    status: OperationStatus::Ok as i32,
                    error_msg: None
                }
            }
            Some(CommandType::Unsub(unsub_cmd)) => {
                if unsub_cmd.events.contains(&(EventType::VarValues as i32)) {
                    for _id in unsub_cmd.var_ids {
                        subscribed_set.remove(&_id);
                    }
                }
                if unsub_cmd.events.contains(&(EventType::TreeChange as i32)) {
                    *get_tree_changes = false;
                }

                Response {
                    response_type: Some(ResponseType::Unsub(UnsubscribeResponse { cmd_id: unsub_cmd.cmd_id })),
                    status: OperationStatus::Ok as i32,
                    error_msg: None
                }
            }
            None => {
                let uid = Uuid::new_v4().to_string();
                Response {
                    response_type: Some(ResponseType::Inv(InvalidCmdResponse { cmd_id: uid })),
                    status: OperationStatus::Err as i32,
                    error_msg: Some("No valid command received".to_string())
                }
            }
        }
    }
}
