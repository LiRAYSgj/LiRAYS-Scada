use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use log::{debug, info, warn};
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::task::{JoinHandle, block_in_place};
use uuid::Uuid;
use prost::Message;
use sled::{Tree, Batch};
use dashmap::DashMap;
use tokio::sync::mpsc::error::TrySendError;
use std::sync::Arc;

// use crate::rtdata::namespace::{AddCommand, ListCommand, SetCommand, SubscribeCommand};

use super::parser::{parse_repeated_name, clone_name};
use super::events::{extract_add_event, extract_del_event};
use super::metrics::Metrics;
use super::utils::{
    CachedValue,
    normalize_path,
    get_ancestors
};
use crate::rtdata::namespace::{
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
    EventBatch,
    EditMetaResponse,
    value::Typed,
    event::Ev,
    command::CommandType,
    response::ResponseType,
    namespace_node::Node,
};

pub struct VariableManager {
    pub items_tree: Tree,
    pub values_tree: Tree,
    pub values_cache: RwLock<HashMap<String, CachedValue>>,
    event_senders: DashMap<u64, mpsc::Sender<Arc<EventBatch>>>,
    next_listener_id: AtomicU64,
    metrics: Arc<Metrics>,
    dirty_values: Mutex<HashMap<String, Option<Typed>>>,
}

impl VariableManager {

    pub fn new(files_dir: &str, metrics: Arc<Metrics>) -> Self {
        let db = sled::open(files_dir).unwrap();
        let items_tree = db.open_tree("mainTree").unwrap();
        let values_tree = db.open_tree("valuesTree").unwrap();
        let values_cache = RwLock::new(HashMap::new());

        let vm = Self {
            items_tree,
            values_tree,
            values_cache,
            event_senders: DashMap::new(),
            next_listener_id: AtomicU64::new(0),
            metrics,
            dirty_values: Mutex::new(HashMap::new()),
        };
        vm.load_cache_from_storage();
        vm
    }

    /// Build `values_cache` from item metadata and persisted values on startup.
    fn load_cache_from_storage(&self) {
        let mut cache = block_in_place(|| self.values_cache.blocking_write());
        for result in self.items_tree.iter() {
            if let Ok((key, value)) = result {
                if let Ok(i_meta) = ItemMeta::decode(value.as_ref()) {
                    if i_meta.i_type() == ItemType::Variable {
                        let key_str = String::from_utf8_lossy(&key);
                        let (parent, _) = match key_str.rsplit_once("/\\0") {
                            Some((p, _)) => (p.to_string(), i_meta.name.clone()),
                            None => continue,
                        };
                        let full_path = format!("{}/{}", parent, i_meta.name);
                        let persisted_val = self
                            .values_tree
                            .get(full_path.as_bytes())
                            .ok()
                            .flatten()
                            .and_then(|v| Value::decode(v.as_ref()).ok())
                            .and_then(|vv| vv.typed);

                        cache.insert(full_path, CachedValue {
                            val: persisted_val,
                            dtype: i_meta.var_d_type(),
                            min: i_meta.min,
                            max: i_meta.max,
                            options: i_meta.options.clone(),
                            max_len: i_meta.max_len,
                        });
                    }
                }
            }
        }
    }

    // ===================== Event dispatching =====================
    /// Register a new subscriber; returns an mpsc receiver of shared `EventBatch`.
    /// Each listener gets its own bounded queue; backlog drops are handled per client.
    pub async fn register_listener(&self) -> mpsc::Receiver<Arc<EventBatch>> {
        let (tx, rx) = mpsc::channel(256);
        let id = self.next_listener_id.fetch_add(1, Ordering::Relaxed);
        self.event_senders.insert(id, tx);
        rx
    }

    /// Fan-out an event batch to all listeners using non-blocking send.
    /// Drops the batch for slow/full clients; removes closed listeners.
    async fn broadcast_batch(&self, batch: Arc<EventBatch>) {
        let mut closed_keys = Vec::new();
        for entry in self.event_senders.iter() {
            match entry.value().try_send(batch.clone()) {
                Ok(_) => {}
                Err(TrySendError::Closed(_)) => closed_keys.push(*entry.key()),
                Err(TrySendError::Full(_)) => {
                    // Drop for this client; optionally log at debug to avoid noise.
                    debug!("Event queue full for listener {}, dropping batch", entry.key());
                }
            }
        }
        if self.metrics.enabled() {
            self.metrics.event_batches.fetch_add(1, Ordering::Relaxed);
            if !closed_keys.is_empty() {
                self.metrics.event_closed.fetch_add(closed_keys.len() as u64, Ordering::Relaxed);
            }
        }
        for k in closed_keys {
            self.event_senders.remove(&k);
        }
    }

    /// Flush dirty variable values to `values_tree` (coalesced last write wins).
    pub async fn flush_dirty_now(&self) {
        let mut dirty_guard = self.dirty_values.lock().await;
        if dirty_guard.is_empty() { return; }
        let mut batch = Batch::default();
        for (var_id, val_opt) in dirty_guard.drain() {
            match val_opt {
                Some(typed) => {
                    let val = Value { typed: Some(typed) };
                    batch.insert(var_id.as_bytes(), val.encode_to_vec());
                }
                None => {
                    batch.remove(var_id.as_bytes());
                }
            }
        }
        if let Err(e) = self.values_tree.apply_batch(batch) {
            warn!("flush_dirty_now: apply_batch error {e}");
        }
        if let Err(e) = self.values_tree.flush_async().await {
            warn!("flush_dirty_now: flush_async error {e}");
        }
    }

    /// Spawn periodic flush of dirty values. Interval in ms.
    pub fn start_flush_loop(self: Arc<Self>, interval_ms: u64) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(interval_ms));
            loop {
                interval.tick().await;
                self.flush_dirty_now().await;
            }
        })
    }

    // ===================== Methods to handle static namespace structure =====================
    // root folders example: "/\0folder1": <meta>, "/\0folder2": <meta>
    // root variables example:  "/\0var1": <meta>, "/\0var2": <meta>
    // deep folders example: "/folder1/\0sub-folder": <meta>, "/folder1/sub-folder/\0sub-folder2": <meta>
    // deep variables example: "/folder1/\0sub-var1": <meta>, "/folder1/sub-folder/\0sub-var2": <meta>

    /// Create folders/variables under `parent_id`, auto-creating missing ancestors,
    /// and seed `values_cache` with metadata for each new variable.
    /// Uses key `parent/\0name` in `items_tree`; `values_cache` keyed by full path without `\0`.
    async fn add_items(
        &self,
        parent_id: &str,
        items_meta: Vec<ItemMeta>,
        batch: &mut Batch
    ) -> Result<(bool, Vec<ItemMeta>, Vec<ItemMeta>), String> {
        let parent_path = normalize_path(parent_id);
        let ancestors = get_ancestors(&parent_path);
        let mut new_folders = Vec::new();
        let mut new_variables = Vec::new();

        for (parent, folder_name) in ancestors {
            let h_key = format!("{}/\0{}", parent, folder_name);
            if !self.items_tree.contains_key(&h_key).map_err(|e| format!("Reading error: {e}"))? {
                let item = ItemMeta {
                    name: folder_name,
                    i_type: ItemType::Folder as i32,
                    var_d_type: None,
                    unit: None,
                    min: None,
                    max: None,
                    options: Vec::new(),
                    max_len: None,
                };
                batch.insert(h_key.as_bytes(), item.encode_to_vec());
            }
        }
        // Create a context to force guard to be drop.
        {
            let mut val_cache_guard = self.values_cache.write().await;
            for i_meta in items_meta {
                // Reject names with reserved characters
                if i_meta.name.contains('/') || i_meta.name.contains('\0') {
                    return Err("Item name cannot contain '/' or NUL".to_string());
                }
                let h_key = format!("{}/\0{}", parent_path, i_meta.name);
                if self.items_tree.contains_key(&h_key).map_err(|e| format!("Reading error: {e}"))? {
                    return Err(format!("Can't create existing item {}", h_key));
                }
                match i_meta.i_type() {
                    ItemType::Folder => {
                        batch.insert(h_key.as_bytes(), i_meta.encode_to_vec());
                        new_folders.push(i_meta)
                    }
                    ItemType::Variable => {
                        let d_key = format!("{}/{}", parent_path, i_meta.name);
                        batch.insert(h_key.as_bytes(), i_meta.encode_to_vec());
                        val_cache_guard.insert(d_key, CachedValue {
                            val: None,
                            dtype: i_meta.var_d_type(),
                            min: i_meta.min,
                            max: i_meta.max,
                            options: i_meta.options.clone(),
                            max_len: i_meta.max_len,
                        });
                        new_variables.push(i_meta)
                    }
                    ItemType::Invalid => return Err("Invalid item type in create".to_string())
                }
            }
        }
        Ok((false, new_folders, new_variables))
    }

    /// Update variable metadata (unit/min/max/options/max_len) and mirror changes in cache.
    /// Does not alter type or name. Requires presence in `items_tree` and `values_cache`.
    async fn edit_variables(
        &self,
        var_id: &str,
        patch_unit: Option<String>,
        patch_min: Option<f64>,
        patch_max: Option<f64>,
        patch_options: Vec<String>,
        patch_max_len: Option<u64>
    ) -> Result<(), String> {
        let normalized = normalize_path(var_id);
        let (parent, name) = normalized.rsplit_once('/').ok_or("Invalid normalized path".to_string())?;
        let h_key = format!("{}/\0{}", parent, name);
        let mut current: ItemMeta = match self.items_tree.get(h_key.as_bytes()) {
            Ok(Some(value)) => ItemMeta::decode(value.as_ref()).map_err(|e| format!("Error decoding Item: {e}"))?,
            Ok(None) => return Err("Variable not found".to_string()),
            Err(e) => return Err(format!("Error reading tree: {e}")),
        };
        let mut val_cache_guard = self.values_cache.write().await;
        let current_cached = val_cache_guard.get_mut(&normalized).ok_or("Variable cache not found".to_string())?;
        if current.i_type() != ItemType::Variable {
            return Err("Only variables support metadata updates".to_string());
        }
        match current.var_d_type() {
            VarDataType::Float | VarDataType::Integer => {
                current.unit = patch_unit;
                current.min = patch_min;
                current.max = patch_max;
                current_cached.min = patch_min;
                current_cached.max = patch_max;
            }
            VarDataType::Text => {
                current.unit = patch_unit;
                current.options = patch_options.clone();
                current.max_len = patch_max_len;
                current_cached.options = patch_options;
                current_cached.max_len = patch_max_len;
            }
            VarDataType::Boolean => {
                current.unit = patch_unit;
            }
            VarDataType::Invalid => warn!("Invalid variable stored: {}", normalized)
        }
        self.items_tree.insert(h_key.as_bytes(), current.encode_to_vec()).map_err(|e| format!("Error Applying batch op: {e}"))?;
        Ok(())
    }

    /// List direct children of `parent_id` from `items_tree` (key `parent/\0name`).
    /// Skips vars missing `var_d_type`, logs warn, continues.
    async fn list_path(&self, parent_id: &str) -> Result<(Vec<FolderInfo>, Vec<VarInfo>), String> {
        let parent_path = normalize_path(parent_id);
        let prefix = format!("{}/\0", parent_path);
        let mut children_folders = Vec::new();
        let mut children_vars = Vec::new();

        for result in self.items_tree.scan_prefix(prefix) {
            let (_, value) = result.map_err(|e| format!("Error reading tree: {e}"))?;
            let i_meta = ItemMeta::decode(value.as_ref()).map_err(|e| format!("Error decoding Item: {e}"))?;
            let i_type = i_meta.i_type();
            let ch_id = format!("{}/{}", parent_path, i_meta.name);
            match i_type {
                ItemType::Folder => {
                    children_folders.push(FolderInfo {id: ch_id, name: i_meta.name});
                }
                ItemType::Variable => {
                    let Some(v_dtype) = i_meta.var_d_type else {
                        warn!("list_path: variable '{}' without var_d_type, skipping", ch_id);
                        continue;
                    };
                    children_vars.push(VarInfo {
                        id: ch_id,
                        name: i_meta.name,
                        var_d_type: v_dtype,
                        unit: i_meta.unit,
                        min: i_meta.min,
                        max: i_meta.max,
                        options: i_meta.options,
                        max_len: i_meta.max_len,
                    });
                }
                _ => return Err("Invalid item type".to_string())
            }
        }

        Ok((children_folders, children_vars))
    }

    /// Remove items (folders or vars) and descendants from `items_tree`,
    /// and purge `values_cache` for those paths. Does not touch `values_tree` yet.
    async fn del_items(&self, item_ids: Vec<String>) -> Result<(), String> {
        let mut batch = Batch::default();
        for id in item_ids {
            let item_path = normalize_path(&id);
            let (parent, name) = item_path.rsplit_once('/').ok_or("Invalid normalized path".to_string())?;
            let h_key = format!("{}/\0{}", parent, name);
            let prefix = format!("{}/", item_path);

            // Remove var value and children variable values
            let mut vars_to_remove: Vec<String> = self.values_cache.read().await
                .keys()
                .filter(|k| k.starts_with(&prefix))
                .cloned()
                .collect();
            vars_to_remove.push(item_path);
            {
                let mut val_cache_guard = self.values_cache.write().await;
                for key in vars_to_remove { val_cache_guard.remove(&key); }
            }

            batch.remove(h_key.as_bytes());  // Remove current reference

            // Remove All potential children references
            for result in self.items_tree.scan_prefix(prefix) {
                let (key, _) = result.map_err(|e| format!("Error reading tree: {e}"))?;
                batch.remove(key);
            }
        }
        self.items_tree.apply_batch(batch).map_err(|e| format!("Error removing items: {e}"))
    }

    /// (Unused) Recursively insert hierarchy from `NamespaceNode`, collecting first-level nodes.
    async fn add_bulk(
        &self,
        root_parent_id: &str,
        root_nodes: HashMap<String, NamespaceNode>,
        batch: &mut Batch
    ) -> Result<(Vec<ItemMeta>, Vec<ItemMeta>), String> {
        let mut stack = vec![(root_parent_id.to_string(), root_nodes)];
        let mut total_count = 0usize;
        let mut first_level_folders = Vec::new();
        let mut first_level_variables = Vec::new();

        while let Some((parent_id, nodes)) = stack.pop() {
            let is_first_level = parent_id == root_parent_id;
            for (key, val) in nodes {
                let mut name_ = key.as_str();
                let (start, stop, step, options) = parse_repeated_name(&mut name_);
                let item_names = clone_name(name_, start, stop, step, options);

                let capacity = item_names.len();
                let mut vars_to_create: Vec<ItemMeta> = Vec::with_capacity(capacity);
                let mut folders_to_create: Vec<ItemMeta> = Vec::with_capacity(capacity);
                let mut folder_children: Vec<(String, HashMap<String, NamespaceNode>)> = Vec::with_capacity(capacity);

                if let Some(node) = val.node {
                    match node {
                        Node::Folder(mut folder) => {
                            for (i, name) in item_names.into_iter().enumerate() {
                                let new_parent_path = if parent_id == "/" {
                                    format!("/{}", name)
                                } else {
                                    format!("{}/{}", parent_id.trim_end_matches('/'), name)
                                };
                                folders_to_create.push(ItemMeta {
                                    name,
                                    i_type: ItemType::Folder as i32,
                                    var_d_type: None,
                                    unit: None,
                                    min: None,
                                    max: None,
                                    options: Vec::new(),
                                    max_len: None,
                                });
                                let children = if i == capacity - 1 {
                                    std::mem::take(&mut folder.children)
                                } else {
                                    folder.children.clone()
                                };
                                folder_children.push((new_parent_path, children));
                            }
                        }
                        Node::Variable(var_def) => {
                            let v_dtype = var_def.var_d_type();
                            if v_dtype == VarDataType::Invalid {
                                warn!("Bulk ADD: Invalid variable type for key '{}'", key);
                            }
                            for name in item_names {
                                vars_to_create.push(ItemMeta {
                                    name,
                                    i_type: ItemType::Variable as i32,
                                    var_d_type: Some(v_dtype as i32),
                                    unit: var_def.unit.clone(),
                                    min: var_def.min,
                                    max: var_def.max,
                                    options: var_def.options.clone(),
                                    max_len: var_def.max_len.clone(),
                                });
                            }
                        }
                    }
                } else {
                    warn!("Bulk ADD: Node with key '{}' has no content defined", key);
                }

                if !vars_to_create.is_empty() {
                    let prev_m = total_count / 1_000_000;
                    let (_, _, new_vars) = self.add_items(&parent_id, vars_to_create, batch).await?;
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
                    let (_, new_folders, _) = self.add_items(&parent_id, folders_to_create, batch).await?;
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

                stack.extend(folder_children);
            }
        }
        Ok((first_level_folders, first_level_variables))
    }

    // ===================== Methods to handle variable values =====================
    // Values are stored in a cache HashMap
    // /folder/path/var1: (varDataType, Option<Typed>)

    /// Apply values to `values_cache` (no persistence), validate against cached metadata,
    /// and build an `EventBatch` for broadcast. Errors if a var is missing in cache.
    async fn set_vals(&self, var_id_vals: Vec<VarIdValue>) -> Result<EventBatch, String> {
        let mut ev_batch = EventBatch { events: Vec::with_capacity(var_id_vals.len()) };
        let mut var_cache_guard = self.values_cache.write().await;
        for var_id_val in var_id_vals {
            let cached_val = match var_cache_guard.get_mut(&var_id_val.var_id) {
                Some(x) => x,
                None => return Err(format!("Variable {} not found", var_id_val.var_id)),
            };
            let new_value = match var_id_val.value {
                Some(value) => Some(self.validate_constraints(cached_val, &value)?),
                None => None
            };
            cached_val.val = new_value.clone();
            // Mark dirty for persistence
            self.dirty_values.lock().await.insert(var_id_val.var_id.clone(), new_value.clone());
            ev_batch.events.push(Event { ev: Some(Ev::VarValueEv(VarIdValue {
                var_id: var_id_val.var_id,
                value: Some(Value { typed: new_value })
            }))});
        }
        Ok(ev_batch)
    }

    /// Read values from `values_cache` only; error if a variable is missing.
    async fn get_vals(&self, var_ids: Vec<String>) -> Result<Vec<OptionalValue>, String> {
        let mut values = Vec::with_capacity(var_ids.len());
        let var_cache_guard = self.values_cache.read().await;
        for var_id in var_ids {
            let typed = var_cache_guard.get(&var_id).ok_or(format!("Variable {var_id} not found"))?.val.clone();
            values.push(OptionalValue { value: Some(Value { typed }) });
        }
        Ok(values)
    }

    pub async fn exec_cmd(
        &self,
        cmd: Command,
        subscribed_set: &mut HashSet<String>,
        get_tree_changes: &mut bool,
    ) -> Response {
        match cmd.command_type {
            Some(CommandType::Add(add_cmd)) => {
                let cmd_id = add_cmd.cmd_id.clone();
                let parent_id = add_cmd.parent_id.clone().unwrap_or_else(|| "/".to_string());
                let mut batch = Batch::default();
                let t0 = std::time::Instant::now();
                let (status, error_msg) = match self.add_items(&parent_id, add_cmd.items_meta.clone(), &mut batch).await {
                    Ok((reload, new_folders, new_variables)) => {
                        match self.items_tree.apply_batch(batch) {
                            Ok(_) => {
                                let folder_id = add_cmd.parent_id.unwrap_or("/".to_string());
                                if let Ok(event) = extract_add_event(&folder_id, reload, new_folders, new_variables) {
                                    let _ = self.broadcast_batch(Arc::new(EventBatch { events: vec![event] })).await;
                                } else {
                                    warn!("Error extracting add event");
                                }
                                (OperationStatus::Ok as i32, None)
                            }
                            Err(e) => (OperationStatus::Err as i32, Some(format!("Applying batch error: {e}")))
                        }
                    }
                    Err(e) => (OperationStatus::Err as i32, Some(format!("Inserting error: {e}")))
                };
                self.metrics.record_add(t0.elapsed());
                Response { response_type: Some(ResponseType::Add(AddResponse { cmd_id })), status, error_msg }
            }
            Some(CommandType::List(list_cmd)) => {
                let folder_to_list = list_cmd.folder_id.unwrap_or_else(|| "/".to_string());
                let t0 = std::time::Instant::now();
                let (status, error_msg, folders, variables) = match self.list_path(&folder_to_list).await {
                    Ok((children_folders, children_vars)) => {
                        (OperationStatus::Ok as i32, None, children_folders, children_vars)
                    }
                    Err(e) => (OperationStatus::Err as i32, Some(e), Vec::new(), Vec::new())
                };
                self.metrics.record_list(t0.elapsed());
                Response { response_type: Some(ResponseType::List(ListResponse { cmd_id: list_cmd.cmd_id, folders, variables })), status, error_msg }
            }
            Some(CommandType::Set(set_cmd)) => {
                let t0 = std::time::Instant::now();
                let (status, error_msg) = match self.set_vals(set_cmd.var_ids_values).await {
                    Ok(batch) => {
                        let _ = self.broadcast_batch(Arc::new(batch)).await;
                        (OperationStatus::Ok as i32, None)
                    }
                    Err(e) => (OperationStatus::Err as i32, Some(e))
                };
                self.metrics.record_set(t0.elapsed());
                Response { response_type: Some(ResponseType::Set(SetResponse { cmd_id: set_cmd.cmd_id })), status, error_msg }
            }
            Some(CommandType::Get(get_cmd)) => {
                let t0 = std::time::Instant::now();
                let (status, error_msg, var_values) = match self.get_vals(get_cmd.var_ids).await {
                    Ok(vals) => (OperationStatus::Ok as i32, None, vals),
                    Err(e) => (OperationStatus::Err as i32, Some(e), Vec::new())
                };
                self.metrics.record_get(t0.elapsed());
                Response { response_type: Some(ResponseType::Get(GetResponse { cmd_id: get_cmd.cmd_id, var_values })), status, error_msg }
            }
            Some(CommandType::Del(del_cmd)) => {
                let t0 = std::time::Instant::now();
                let (status, error_msg) = match self.del_items(del_cmd.item_ids.clone()).await {
                    Ok(_) => {
                        if let Ok(event) = extract_del_event(&del_cmd) {
                            let _ = self.broadcast_batch(Arc::new(EventBatch { events: vec![event] })).await;
                        } else {
                            warn!("Error extracting event");
                        }
                        (OperationStatus::Ok as i32, None)
                    }
                    Err(e) => (OperationStatus::Err as i32, Some(e))
                };
                self.metrics.record_del(t0.elapsed());
                Response { response_type: Some(ResponseType::Del(DelResponse { cmd_id: del_cmd.cmd_id })), status, error_msg }
            }
            Some(CommandType::AddBulk(addb_cmd)) => {
                let cmd_id = addb_cmd.cmd_id.clone();
                let parent_id = normalize_path(&addb_cmd.parent_id);
                let mut batch = sled::Batch::default();
                let t0 = std::time::Instant::now();
                let (status, error_msg) = match addb_cmd.schema {
                    Some(schema) => match self.add_bulk(&parent_id, schema.roots, &mut batch).await {
                        Ok((new_folders, new_variables)) => match self.items_tree.apply_batch(batch) {
                            Ok(_) => {
                                if let Ok(event) = extract_add_event(&parent_id, false, new_folders, new_variables) {
                                    let _ = self.broadcast_batch(Arc::new(EventBatch { events: vec![event] })).await;
                                } else {
                                    warn!("Error extracting event");
                                }
                                (OperationStatus::Ok as i32, None)
                            }
                            Err(e) => (OperationStatus::Err as i32, Some(format!("On apply batch: {e}"))),
                        },
                        Err(e) => (OperationStatus::Err as i32, Some(format!("Bulk insert error: {e}"))),
                    },
                    None => (OperationStatus::Err as i32, Some("No schema provided".to_string())),
                };
                self.metrics.record_add(t0.elapsed());
                Response { response_type: Some(ResponseType::AddBulk(AddBulkResponse { cmd_id })), status, error_msg }
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
            Some(CommandType::EditMeta(edit_cmd)) => {
                let (status, error_msg) = match self.edit_variables(
                    &edit_cmd.var_id,
                    edit_cmd.unit,
                    edit_cmd.min,
                    edit_cmd.max,
                    edit_cmd.options,
                    edit_cmd.max_len
                ).await {
                    Ok(_) => (OperationStatus::Ok as i32, None),
                    Err(e) => (OperationStatus::Err as i32, Some(e)),
                };
                Response {
                    response_type: Some(ResponseType::EditMeta(EditMetaResponse { cmd_id: edit_cmd.cmd_id })),
                    status,
                    error_msg,
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

    /// Validate an incoming `Value` against cached dtype/constraints and return normalized `Typed`.
    /// - Numeric: checks min/max; preserves integer dtype (casts float input to i64).
    /// - Text: checks options/max_len.
    /// - Boolean: accepts bool or non-zero int/float as true.
    fn validate_constraints(&self, cached_val: &CachedValue, value: &Value) -> Result<Typed, String> {
        let typed = value
            .typed
            .as_ref()
            .ok_or_else(|| "Missing value".to_string())?.to_owned();

        match cached_val.dtype {
            VarDataType::Float | VarDataType::Integer => {
                // Allow int or float input; ensure bounds against min/max (as f64),
                // but preserve integer dtype when dtype == Integer.
                let v_f64 = match typed {
                    Typed::FloatValue(x) => x,
                    Typed::IntegerValue(x) => x as f64,
                    _ => return Err("Type mismatch for numeric variable".to_string()),
                };
                if let Some(min) = cached_val.min { if v_f64 < min { return Err(format!("Value {} is below min {}", v_f64, min)); }}
                if let Some(max) = cached_val.max { if v_f64 > max { return Err(format!("Value {} is above max {}", v_f64, max)); }}
                match cached_val.dtype {
                    VarDataType::Integer => Ok(Typed::IntegerValue(v_f64 as i64)),
                    VarDataType::Float => Ok(Typed::FloatValue(v_f64)),
                    _ => unreachable!(),
                }
            }
            VarDataType::Text => {
                let v = match typed {
                    Typed::TextValue(x) => x,
                    _ => return Err("Type mismatch for text variable".to_string()),
                };
                if !cached_val.options.is_empty() && !cached_val.options.contains(&v) {
                    return Err("Text value not in allowed options".to_string());
                }
                if let Some(limit) = cached_val.max_len {
                    if (v.chars().count() as u64) > limit {
                        return Err(format!("Text length exceeds max {}", limit));
                    }
                }
                Ok(Typed::TextValue(v))
            }
            VarDataType::Boolean => {
                let v = match typed {
                    Typed::BooleanValue(x) => x,
                    Typed::FloatValue(x) => x != 0.0,
                    Typed::IntegerValue(x) => x != 0,
                    _ => return Err("Type mismatch for boolean variable".to_string()),
                };
                Ok(Typed::BooleanValue(v))
            }
            VarDataType::Invalid => return Err("Invalid var data type.".to_string()),
        }
    }
}
