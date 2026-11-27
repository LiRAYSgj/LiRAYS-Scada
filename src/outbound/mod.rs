use std::{future::Future, pin::Pin, sync::Arc};

use lirays_scada_proto::namespace::v1::VarIdValue;
use serde_json::Value;

use crate::plugin_form::PluginDescriptor;

#[cfg(feature = "udp")]
pub mod udp;

/// Latest values of the selected variables, stamped when the batch was assembled.
pub struct OutboundBatch {
    pub timestamp_ms: u64,
    pub values: Vec<VarIdValue>,
}

/// One configured destination. The engine runs at most one publish per instance at a
/// time and retries failed batches. Publishing must be cancellation-safe.
pub trait OutboundConnection: Send {
    fn publish<'a>(
        &'a mut self,
        batch: &'a OutboundBatch,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
}

/// Compiled-in factory. Receives only the instance `target` configuration: variable
/// selection and read mode belong to the engine. Validate without starting background work.
pub trait OutboundPlugin: Send + Sync {
    fn kind(&self) -> &'static str;
    fn create(
        &self,
        instance_id: &str,
        target: &Value,
    ) -> Result<Box<dyn OutboundConnection>, String>;

    /// The form for the `target` configuration. Without one, clients edit raw JSON.
    fn descriptor(&self) -> PluginDescriptor {
        PluginDescriptor::new(self.kind())
    }
}

pub fn builtin_plugins() -> Vec<Arc<dyn OutboundPlugin>> {
    vec![
        #[cfg(feature = "udp")]
        Arc::new(udp::UdpPlugin),
    ]
}
