mod runner;
pub mod service;

use std::{future::Future, sync::Arc};

use axum::{Router, middleware::from_fn_with_state, routing::get};
use serde_json::Value;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use super::plugin_instances::{InstanceStatus, PluginKind};
use crate::{
    egress::EgressPlugin,
    http::{AppState, resources::auth::middleware::admin_only},
    rtdata::variable::VariableManager,
};

pub struct Egressors;

impl PluginKind for Egressors {
    type Plugin = dyn EgressPlugin;
    type Prepared = runner::Instance;

    const TABLE: &'static str = "egressors";

    fn plugin_type(plugin: &Self::Plugin) -> &'static str {
        plugin.kind()
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
            .route("/api/egressors", get(service::list).post(service::create))
            .route("/api/egressors/plugins", get(service::plugins))
            .route(
                "/api/egressors/{id}",
                get(service::get)
                    .put(service::update)
                    .delete(service::delete),
            )
            .route_layer(from_fn_with_state(state, admin_only)),
    )
}
