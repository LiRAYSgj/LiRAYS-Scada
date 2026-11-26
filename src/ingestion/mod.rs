use std::{future::Future, pin::Pin, sync::Arc};

use lirays_scada_proto::namespace::v1::VarIdValue;
use serde_json::Value;

#[cfg(feature = "simulator")]
pub mod simulator;

/// A batch of canonical namespace paths and typed values, without variable definitions.
pub struct IngestionBatch {
    pub values: Vec<VarIdValue>,
}

/// One configured source. Wait for data rather than returning empty batches in a busy loop.
/// Pending reads must be cancellation-safe: disabling an instance drops the read future.
pub trait Ingestor: Send {
    fn next_batch(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<IngestionBatch, String>> + Send + '_>>;
}

/// Compiled-in factory. Validate configuration here without starting background work.
/// Instance IDs remain stable across configuration updates and restarts.
pub trait IngestorPlugin: Send + Sync {
    fn kind(&self) -> &'static str;
    fn create(&self, instance_id: &str, config: &Value) -> Result<Box<dyn Ingestor>, String>;
}

pub fn builtin_plugins() -> Vec<Arc<dyn IngestorPlugin>> {
    vec![
        #[cfg(feature = "simulator")]
        Arc::new(simulator::SimulatorPlugin),
    ]
}
