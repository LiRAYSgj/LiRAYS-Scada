use std::{future::Future, pin::Pin, sync::Arc};

use lirays_scada_proto::namespace::v1::VarIdValue;
use serde_json::Value;

use crate::plugin_form::PluginDescriptor;

#[cfg(feature = "simulator")]
pub mod simulator;

/// A batch of canonical namespace paths and typed values, without variable definitions.
pub struct InboundBatch {
    pub values: Vec<VarIdValue>,
}

/// Parameters of one variable, keyed by name.
pub type VariableParams = serde_json::Map<String, Value>;

/// One configured source. Wait for data rather than returning empty batches in a busy loop.
/// Pending reads must be cancellation-safe: disabling an instance drops the read future.
pub trait InboundConnection: Send {
    fn next_batch(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<InboundBatch, String>> + Send + '_>>;

    /// Parameters of the variable at `path`, read only when an inbound batch creates
    /// that variable. Only the keys named exactly `min` and `max`, with numeric values,
    /// are applied, and only to numeric variables; any other key (for example `maximum`)
    /// is ignored. Existing variables keep their metadata.
    fn variable_params(&self, _path: &str) -> Option<VariableParams> {
        None
    }
}

/// Compiled-in factory. Validate configuration here without starting background work.
/// Instance IDs remain stable across configuration updates and restarts.
pub trait InboundPlugin: Send + Sync {
    fn kind(&self) -> &'static str;
    fn create(
        &self,
        instance_id: &str,
        config: &Value,
    ) -> Result<Box<dyn InboundConnection>, String>;

    /// The configuration form shown to clients. Without one, clients edit raw JSON.
    fn descriptor(&self) -> PluginDescriptor {
        PluginDescriptor::new(self.kind())
    }
}

pub fn builtin_plugins() -> Vec<Arc<dyn InboundPlugin>> {
    vec![
        #[cfg(feature = "simulator")]
        Arc::new(simulator::SimulatorPlugin),
    ]
}
