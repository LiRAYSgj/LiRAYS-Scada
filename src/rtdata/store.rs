use std::future::Future;

use sled::transaction::Transactional;
use sled::{Batch, IVec, Tree};

/// Ordered writes for one keyspace; when a key repeats, the last operation wins.
#[derive(Default)]
pub struct WriteBatch {
    ops: Vec<(Vec<u8>, Option<Vec<u8>>)>,
}

impl WriteBatch {
    pub fn insert(&mut self, key: impl Into<Vec<u8>>, value: Vec<u8>) {
        self.ops.push((key.into(), Some(value)));
    }

    pub fn remove(&mut self, key: impl Into<Vec<u8>>) {
        self.ops.push((key.into(), None));
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

pub struct SledStore {
    pub(super) items: Tree,
    pub(super) values: Tree,
}

impl SledStore {
    pub fn open(dir: &str) -> Result<Self, String> {
        let db = sled::open(dir).map_err(|e| format!("Failed to open variable DB store: {e}"))?;
        let items = db
            .open_tree("mainTree")
            .map_err(|e| format!("Failed to open items tree: {e}"))?;
        let values = db
            .open_tree("valuesTree")
            .map_err(|e| format!("Failed to open values tree: {e}"))?;
        Ok(Self { items, values })
    }
}

fn sled_batch(batch: WriteBatch) -> Batch {
    let mut sled_batch = Batch::default();
    for (key, value) in batch.ops {
        match value {
            Some(value) => sled_batch.insert(key, value),
            None => sled_batch.remove(key),
        }
    }
    sled_batch
}

impl VariableStore for SledStore {
    type Bytes = IVec;

    fn items(&self) -> impl Iterator<Item = Result<(IVec, IVec), String>> + '_ {
        self.items.iter().map(|r| r.map_err(|e| e.to_string()))
    }

    fn scan_items(&self, prefix: &[u8]) -> impl Iterator<Item = Result<(IVec, IVec), String>> + '_ {
        self.items
            .scan_prefix(prefix)
            .map(|r| r.map_err(|e| e.to_string()))
    }

    fn get_item(&self, key: &[u8]) -> Result<Option<IVec>, String> {
        self.items.get(key).map_err(|e| e.to_string())
    }

    fn contains_item(&self, key: &[u8]) -> Result<bool, String> {
        self.items.contains_key(key).map_err(|e| e.to_string())
    }

    fn apply_items(&self, batch: WriteBatch) -> Result<(), String> {
        self.items
            .apply_batch(sled_batch(batch))
            .map_err(|e| e.to_string())
    }

    fn scan_value_keys(&self, prefix: &[u8]) -> impl Iterator<Item = Result<IVec, String>> + '_ {
        self.values
            .scan_prefix(prefix)
            .keys()
            .map(|r| r.map_err(|e| e.to_string()))
    }

    fn get_value(&self, key: &[u8]) -> Result<Option<IVec>, String> {
        self.values.get(key).map_err(|e| e.to_string())
    }

    async fn write_values(&self, batch: WriteBatch) -> Result<(), String> {
        let tree = self.values.clone();
        tokio::task::spawn_blocking(move || tree.apply_batch(sled_batch(batch)))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        self.values
            .flush_async()
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn apply_atomic(&self, items: WriteBatch, values: WriteBatch) -> Result<(), String> {
        let items = sled_batch(items);
        let values = sled_batch(values);
        let result: Result<(), sled::transaction::TransactionError<()>> =
            (&self.items, &self.values).transaction(|(items_tx, values_tx)| {
                items_tx.apply_batch(&items)?;
                values_tx.apply_batch(&values)?;
                Ok(())
            });
        result.map_err(|e| format!("{e:?}"))
    }
}
