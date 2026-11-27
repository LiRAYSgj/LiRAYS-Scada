use std::collections::BTreeMap;
use std::future::Future;

use std::path::Path;

/// Writes for one keyspace, one operation per key; when a key repeats, the last one wins.
#[derive(Default)]
pub struct WriteBatch {
    ops: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
}

impl WriteBatch {
    pub fn insert(&mut self, key: impl Into<Vec<u8>>, value: Vec<u8>) {
        self.ops.insert(key.into(), Some(value));
    }

    pub fn remove(&mut self, key: impl Into<Vec<u8>>) {
        self.ops.insert(key.into(), None);
    }
}

/// Persistent storage of one real-time namespace.
///
/// It holds two ordered keyspaces: namespace items keyed by `parent/\0name` with encoded
/// `ItemMeta`, and variable values keyed by full path with encoded `Value`. Prefix scans
/// return keys in byte order. Each instance owns its own directory, so several stores can
/// be open in the same process.
pub trait VariableStore: Send + Sync + 'static {
    type Bytes: AsRef<[u8]> + Send;

    fn items(&self) -> impl Iterator<Item = Result<(Self::Bytes, Self::Bytes), String>> + '_;

    fn scan_items(
        &self,
        prefix: &[u8],
    ) -> impl Iterator<Item = Result<(Self::Bytes, Self::Bytes), String>> + '_;

    fn get_item(&self, key: &[u8]) -> Result<Option<Self::Bytes>, String>;

    fn contains_item(&self, key: &[u8]) -> Result<bool, String>;

    fn apply_items(&self, batch: WriteBatch) -> Result<(), String>;

    fn scan_value_keys(
        &self,
        prefix: &[u8],
    ) -> impl Iterator<Item = Result<Self::Bytes, String>> + '_;

    fn get_value(&self, key: &[u8]) -> Result<Option<Self::Bytes>, String>;

    /// Apply value writes and wait until they are durable, without blocking the runtime.
    fn write_values(&self, batch: WriteBatch) -> impl Future<Output = Result<(), String>> + Send;

    /// Apply item and value writes atomically.
    fn apply_atomic(&self, items: WriteBatch, values: WriteBatch) -> Result<(), String>;
}

/// Opens a fjall database, refusing directories left by the former sled engine.
pub fn open_database(dir: &str) -> Result<fjall::Database, String> {
    let path = Path::new(dir);
    if path.join("conf").is_file() && path.join("db").is_file() {
        return Err(format!(
            "'{dir}' holds data from the former sled storage engine, which is no longer supported; \
             move or delete it to start with an empty store"
        ));
    }
    fjall::Database::builder(path)
        .open()
        .map_err(|e| format!("Failed to open store at '{dir}': {e}"))
}

/// fjall store: namespace changes are fsynced on commit; value batches are fsynced by
/// `write_values`.
#[derive(Clone)]
pub struct FjallStore {
    db: fjall::Database,
    pub(super) items: fjall::Keyspace,
    pub(super) values: fjall::Keyspace,
}

impl FjallStore {
    pub fn open(dir: &str) -> Result<Self, String> {
        let db = open_database(dir)?;
        let items = db
            .keyspace("items", fjall::KeyspaceCreateOptions::default)
            .map_err(|e| format!("Failed to open items keyspace: {e}"))?;
        let values = db
            .keyspace("values", fjall::KeyspaceCreateOptions::default)
            .map_err(|e| format!("Failed to open values keyspace: {e}"))?;
        Ok(Self { db, items, values })
    }

    fn commit(
        &self,
        writes: impl IntoIterator<Item = (fjall::Keyspace, WriteBatch)>,
        durability: fjall::PersistMode,
    ) -> Result<(), String> {
        let mut batch = self.db.batch().durability(Some(durability));
        for (keyspace, writes) in writes {
            for (key, value) in writes.ops {
                match value {
                    Some(value) => batch.insert(&keyspace, key, value),
                    None => batch.remove(&keyspace, key),
                }
            }
        }
        batch.commit().map_err(|e| e.to_string())
    }
}

fn fjall_pairs(
    iter: fjall::Iter,
) -> impl Iterator<Item = Result<(fjall::Slice, fjall::Slice), String>> {
    iter.map(|guard| guard.into_inner().map_err(|e| e.to_string()))
}

impl VariableStore for FjallStore {
    type Bytes = fjall::Slice;

    fn items(&self) -> impl Iterator<Item = Result<(fjall::Slice, fjall::Slice), String>> + '_ {
        fjall_pairs(self.items.iter())
    }

    fn scan_items(
        &self,
        prefix: &[u8],
    ) -> impl Iterator<Item = Result<(fjall::Slice, fjall::Slice), String>> + '_ {
        fjall_pairs(self.items.prefix(prefix))
    }

    fn get_item(&self, key: &[u8]) -> Result<Option<fjall::Slice>, String> {
        self.items.get(key).map_err(|e| e.to_string())
    }

    fn contains_item(&self, key: &[u8]) -> Result<bool, String> {
        self.items.contains_key(key).map_err(|e| e.to_string())
    }

    fn apply_items(&self, batch: WriteBatch) -> Result<(), String> {
        self.commit([(self.items.clone(), batch)], fjall::PersistMode::SyncAll)
    }

    fn scan_value_keys(
        &self,
        prefix: &[u8],
    ) -> impl Iterator<Item = Result<fjall::Slice, String>> + '_ {
        self.values
            .prefix(prefix)
            .map(|guard| guard.key().map_err(|e| e.to_string()))
    }

    fn get_value(&self, key: &[u8]) -> Result<Option<fjall::Slice>, String> {
        self.values.get(key).map_err(|e| e.to_string())
    }

    async fn write_values(&self, batch: WriteBatch) -> Result<(), String> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || {
            store.commit([(store.values.clone(), batch)], fjall::PersistMode::SyncAll)
        })
        .await
        .map_err(|e| e.to_string())?
    }

    fn apply_atomic(&self, items: WriteBatch, values: WriteBatch) -> Result<(), String> {
        self.commit(
            [(self.items.clone(), items), (self.values.clone(), values)],
            fjall::PersistMode::SyncAll,
        )
    }
}
