use std::{future::Future, pin::Pin, sync::Arc};

use lirays_scada_proto::namespace::v1::VarIdValue;
use serde_json::Value;

#[cfg(feature = "udp")]
pub mod udp;

/// Latest values of the selected variables, stamped when the batch was assembled.
pub struct EgressBatch {
    pub timestamp_ms: u64,
    pub values: Vec<VarIdValue>,
}

/// One configured destination. The engine runs at most one publish per instance at a
/// time and retries failed batches. Publishing must be cancellation-safe.
pub trait Egressor: Send {
    fn publish<'a>(
        &'a mut self,
        batch: &'a EgressBatch,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
}

/// Compiled-in factory. Receives only the instance `target` configuration: variable
/// selection and read mode belong to the engine. Validate without starting background work.
pub trait EgressPlugin: Send + Sync {
    fn kind(&self) -> &'static str;
    fn create(&self, instance_id: &str, target: &Value) -> Result<Box<dyn Egressor>, String>;
}

pub fn builtin_plugins() -> Vec<Arc<dyn EgressPlugin>> {
    vec![
        #[cfg(feature = "udp")]
        Arc::new(udp::UdpPlugin),
    ]
}
