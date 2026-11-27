mod runner;
pub mod service;

use std::{future::Future, sync::Arc};

use axum::{Router, middleware::from_fn_with_state, routing::get};
use serde_json::{Value, json};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use super::plugin_instances::{InstanceStatus, PluginKind};
use crate::{
    http::{AppState, resources::auth::middleware::admin_only},
    outbound::OutboundPlugin,
    plugin_form::{FieldKind, FormField, PluginDescriptor},
    rtdata::variable::VariableManager,
};

pub struct OutboundConnections;

impl PluginKind for OutboundConnections {
    type Plugin = dyn OutboundPlugin;
    type Prepared = runner::Instance;

    const TABLE: &'static str = "outbound";
    const LABEL: &'static str = "Outbound";

    fn plugin_type(plugin: &Self::Plugin) -> &'static str {
        plugin.kind()
    }

    /// Wraps the plugin's `target` form with the engine settings every outbound connection shares.
    fn descriptor(plugin: &Self::Plugin) -> PluginDescriptor {
        let target = plugin.descriptor();
        let target_field = match target.fields {
            Some(fields) => FormField::new("target", "Target", FieldKind::Group { fields }),
            None => FormField::new("target", "Target", FieldKind::Json).default_value(json!({})),
        };
        PluginDescriptor {
            fields: Some(vec![
                FormField::new("variables", "Variables", FieldKind::TagList)
                    .required()
                    .folder_as_pattern()
                    .description("Paths or patterns; * matches one segment and ** any depth."),
                FormField::new(
                    "mode",
                    "Read mode",
                    FieldKind::Group {
                        fields: vec![
                            FormField::new(
                                "type",
                                "Mode",
                                FieldKind::select(&[
                                    ("subscription", "On change"),
                                    ("polling", "Polling"),
                                ]),
                            )
                            .required()
                            .default_value(json!("subscription")),
                            FormField::new(
                                "interval_ms",
                                "Interval (ms)",
                                FieldKind::integer(Some(1.0), None),
                            )
                            .required()
                            .default_value(json!(1000))
                            .visible_when("type", vec![json!("polling")]),
                        ],
                    },
                ),
                target_field.required(),
            ]),
            ..target
        }
    }

    fn prepare(
        plugin: &Self::Plugin,
        instance_id: &str,
        config: &Value,
    ) -> Result<Self::Prepared, String> {
        runner::prepare(plugin, instance_id, config)
    }

    fn run(
        instance: Self::Prepared,
        variables: Arc<VariableManager>,
        cancel: CancellationToken,
        status: watch::Sender<InstanceStatus>,
    ) -> impl Future<Output = ()> + Send + 'static {
        runner::run(instance, variables, cancel, status)
    }
}

pub fn mount(router: Router<Arc<AppState>>, state: Arc<AppState>) -> Router<Arc<AppState>> {
    router.merge(
        Router::new()
            .route("/api/outbound", get(service::list).post(service::create))
            .route("/api/outbound/plugins", get(service::plugins))
            .route(
                "/api/outbound/{id}",
                get(service::get)
                    .put(service::update)
                    .delete(service::delete),
            )
            .route_layer(from_fn_with_state(state, admin_only)),
    )
}

#[cfg(all(test, feature = "udp"))]
mod tests {
    use super::*;
    use crate::{outbound::udp::UdpPlugin, plugin_form::default_config};

    #[test]
    fn form_defaults_prepare_in_both_read_modes() {
        let fields = OutboundConnections::descriptor(&UdpPlugin).fields.unwrap();
        let mut config = default_config(&fields);
        config["variables"] = json!(["/plant/**"]);
        config["target"]["host"] = json!("127.0.0.1");
        let subscription = {
            let mut config = config.clone();
            config["mode"]
                .as_object_mut()
                .unwrap()
                .remove("interval_ms");
            config
        };
        config["mode"]["type"] = json!("polling");
        for config in [subscription, config] {
            assert!(
                runner::prepare(&UdpPlugin, "test", &config).is_ok(),
                "Rejected {config}"
            );
        }
    }
}
