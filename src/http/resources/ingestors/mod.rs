pub mod service;

use std::sync::Arc;

use axum::{Router, middleware::from_fn_with_state, routing::get};
use serde_json::Value;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use super::plugin_instances::{InstanceState, InstanceStatus, PluginKind};
use crate::{
    http::{AppState, resources::auth::middleware::admin_only},
    ingestion::{Ingestor, IngestorPlugin},
    rtdata::variable::VariableManager,
};

pub struct Ingestors;

impl PluginKind for Ingestors {
    type Plugin = dyn IngestorPlugin;
    type Prepared = Box<dyn Ingestor>;

    const TABLE: &'static str = "ingestors";

    fn plugin_type(plugin: &Self::Plugin) -> &'static str {
        plugin.kind()
    }

    fn prepare(
        plugin: &Self::Plugin,
        instance_id: &str,
        config: &Value,
    ) -> Result<Self::Prepared, String> {
        plugin.create(instance_id, config)
    }

    /// Applies one batch before requesting the next; a source or batch error stops the instance.
    async fn run(
        mut source: Self::Prepared,
        variables: Arc<VariableManager>,
        cancel: CancellationToken,
        status: watch::Sender<InstanceStatus>,
    ) {
        loop {
            let next = tokio::select! {
                biased;
                _ = cancel.cancelled() => break,
                next = source.next_batch() => next,
            };
            let result = match next {
                Ok(batch) => variables.ingest(batch).await,
                Err(error) => Err(error),
            };
            if let Err(error) = result {
                log::warn!("Ingestor stopped: {error}");
                status.send_replace(InstanceStatus {
                    state: InstanceState::Error,
                    last_error: Some(error),
                });
                return;
            }
        }
        status.send_replace(InstanceStatus {
            state: InstanceState::Stopped,
            last_error: None,
        });
    }
}

pub fn mount(router: Router<Arc<AppState>>, state: Arc<AppState>) -> Router<Arc<AppState>> {
    router.merge(
        Router::new()
            .route("/api/ingestors", get(service::list).post(service::create))
            .route("/api/ingestors/plugins", get(service::plugins))
            .route(
                "/api/ingestors/{id}",
                get(service::get)
                    .put(service::update)
                    .delete(service::delete),
            )
            .route_layer(from_fn_with_state(state, admin_only)),
    )
}
