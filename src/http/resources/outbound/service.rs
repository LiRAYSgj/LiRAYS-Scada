use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use super::OutboundConnections;
use crate::http::{
    ApiResponse, ApiResponseEmpty, AppState,
    resources::plugin_instances::{
        InstanceError, InstanceInfo, InstanceInput, InstanceUpdate, PluginManager,
    },
};

pub type OutboundManager = PluginManager<OutboundConnections>;

#[utoipa::path(get, operation_id = "list_outbound_plugins", path = "/api/outbound/plugins", tag = "Outbound", security(("bearerAuth" = [])),
    responses((status = 200, description = "Compiled-in plugin types", body = [String])))]
pub async fn plugins(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    Json(state.outbound.plugin_types())
}

#[utoipa::path(get, operation_id = "list_outbound", path = "/api/outbound", tag = "Outbound", security(("bearerAuth" = [])),
    responses((status = 200, description = "Outbound connections and runtime status", body = ApiResponse<Vec<InstanceInfo>>)))]
pub async fn list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<InstanceInfo>>>, InstanceError> {
    Ok(Json(ApiResponse::success(state.outbound.list().await?)))
}

#[utoipa::path(get, operation_id = "get_outbound", path = "/api/outbound/{id}", tag = "Outbound", security(("bearerAuth" = [])),
    params(("id" = String, Path, description = "Instance ID")),
    responses((status = 200, description = "Outbound connection", body = ApiResponse<InstanceInfo>), (status = 404, description = "Not found", body = ApiResponseEmpty)))]
pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<InstanceInfo>>, InstanceError> {
    Ok(Json(ApiResponse::success(state.outbound.get(&id).await?)))
}

#[utoipa::path(post, operation_id = "create_outbound", path = "/api/outbound", tag = "Outbound", security(("bearerAuth" = [])),
    request_body(content = InstanceInput, example = json!({"name":"UDP stream","plugin_type":"udp","enabled":true,"config":{"variables":["/plant/**"],"mode":{"type":"subscription"},"target":{"host":"127.0.0.1","port":9000}}})),
    responses((status = 201, description = "Instance created", body = ApiResponse<InstanceInfo>), (status = 400, description = "Invalid configuration", body = ApiResponseEmpty)))]
pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(input): Json<InstanceInput>,
) -> Result<impl IntoResponse, InstanceError> {
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(
            state.outbound.save(None, input).await?,
        )),
    ))
}

#[utoipa::path(put, operation_id = "update_outbound", path = "/api/outbound/{id}", tag = "Outbound", security(("bearerAuth" = [])),
    params(("id" = String, Path, description = "Instance ID")), request_body(content = InstanceUpdate, example = json!({"config":{"mode":{"type":"polling","interval_ms":1000}}})),
    responses((status = 200, description = "Fields merged; omitted fields remain unchanged; arrays replace existing arrays", body = ApiResponse<InstanceInfo>), (status = 400, description = "Invalid configuration", body = ApiResponseEmpty), (status = 404, description = "Not found", body = ApiResponseEmpty)))]
pub async fn update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(input): Json<InstanceUpdate>,
) -> Result<Json<ApiResponse<InstanceInfo>>, InstanceError> {
    Ok(Json(ApiResponse::success(
        state.outbound.save(Some(&id), input).await?,
    )))
}

#[utoipa::path(delete, operation_id = "delete_outbound", path = "/api/outbound/{id}", tag = "Outbound", security(("bearerAuth" = [])),
    params(("id" = String, Path, description = "Instance ID")),
    responses((status = 200, description = "Instance deleted", body = ApiResponseEmpty), (status = 404, description = "Not found", body = ApiResponseEmpty)))]
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponseEmpty>, InstanceError> {
    state.outbound.delete(&id).await?;
    Ok(Json(ApiResponseEmpty::success()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        http::resources::plugin_instances::{InstanceState, PluginKind, ensure_schema},
        inbound::InboundBatch,
        outbound::{OutboundBatch, OutboundConnection, OutboundPlugin},
        rtdata::{metrics::Metrics, variable::VariableManager},
    };
    use lirays_scada_proto::namespace::v1::{Value as ProtoValue, VarIdValue, value::Typed};
    use sea_orm::Database;
    use serde_json::Value;
    use serde_json::json;
    use std::{
        future::Future,
        pin::Pin,
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };
    use tokio::sync::mpsc;
    use uuid::Uuid;

    struct CapturePlugin {
        published: mpsc::UnboundedSender<Vec<(String, i64)>>,
        failures: Arc<AtomicUsize>,
    }

    struct Capture {
        published: mpsc::UnboundedSender<Vec<(String, i64)>>,
        failures: Arc<AtomicUsize>,
    }

    impl OutboundPlugin for CapturePlugin {
        fn kind(&self) -> &'static str {
            "capture"
        }
        fn create(&self, _: &str, target: &Value) -> Result<Box<dyn OutboundConnection>, String> {
            if !target.is_object() {
                return Err("target must be an object".into());
            }
            Ok(Box::new(Capture {
                published: self.published.clone(),
                failures: self.failures.clone(),
            }))
        }
    }

    impl OutboundConnection for Capture {
        fn publish<'a>(
            &'a mut self,
            batch: &'a OutboundBatch,
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
            Box::pin(async move {
                if self
                    .failures
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
                    .is_ok()
                {
                    return Err("destination unavailable".into());
                }
                let mut values: Vec<_> = batch
                    .values
                    .iter()
                    .map(|v| match v.value.as_ref().and_then(|v| v.typed.as_ref()) {
                        Some(Typed::IntegerValue(n)) => (v.var_id.clone(), *n),
                        _ => panic!("unexpected value"),
                    })
                    .collect();
                values.sort();
                let _ = self.published.send(values);
                Ok(())
            })
        }
    }

    struct Setup {
        manager: OutboundManager,
        variables: Arc<VariableManager>,
        published: mpsc::UnboundedReceiver<Vec<(String, i64)>>,
        failures: Arc<AtomicUsize>,
    }

    async fn setup() -> Setup {
        let dir = std::env::temp_dir().join(format!("outbound-test-{}", Uuid::new_v4()));
        let db = Database::connect("sqlite::memory:").await.unwrap();
        ensure_schema(&db, OutboundConnections::TABLE)
            .await
            .unwrap();
        let variables = Arc::new(
            VariableManager::new(
                dir.to_str().unwrap(),
                Arc::new(Metrics::new(dir.join("metrics"), false, false)),
            )
            .unwrap(),
        );
        let (sender, published) = mpsc::unbounded_channel();
        let failures = Arc::new(AtomicUsize::new(0));
        let plugin = CapturePlugin {
            published: sender,
            failures: failures.clone(),
        };
        let manager = OutboundManager::new(db, variables.clone(), vec![Arc::new(plugin)]).unwrap();
        Setup {
            manager,
            variables,
            published,
            failures,
        }
    }

    async fn apply_inbound(variables: &VariableManager, values: &[(&str, i64)]) {
        let values = values
            .iter()
            .map(|(path, n)| VarIdValue {
                var_id: path.to_string(),
                value: Some(ProtoValue {
                    typed: Some(Typed::IntegerValue(*n)),
                }),
            })
            .collect();
        variables
            .apply_inbound(InboundBatch { values })
            .await
            .unwrap();
    }

    async fn next(
        published: &mut mpsc::UnboundedReceiver<Vec<(String, i64)>>,
    ) -> Vec<(String, i64)> {
        tokio::time::timeout(Duration::from_secs(3), published.recv())
            .await
            .unwrap()
            .unwrap()
    }

    fn input(config: Value) -> InstanceInput {
        InstanceInput {
            name: "Capture".into(),
            plugin_type: "capture".into(),
            enabled: true,
            config,
        }
    }

    fn owned(values: &[(&str, i64)]) -> Vec<(String, i64)> {
        values.iter().map(|(p, n)| (p.to_string(), *n)).collect()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn subscription_sends_snapshot_then_matching_changes_including_new_variables() {
        let Setup {
            manager,
            variables,
            mut published,
            ..
        } = setup().await;
        apply_inbound(&variables, &[("/plant/a", 1), ("/other/a", 1)]).await;
        let config = json!({"variables":["/plant/*"],"mode":{"type":"subscription"},"target":{}});
        let instance = manager.save(None, input(config)).await.unwrap();
        assert_eq!(next(&mut published).await, owned(&[("/plant/a", 1)]));

        apply_inbound(&variables, &[("/other/a", 2), ("/plant/b", 5)]).await;
        assert_eq!(next(&mut published).await, owned(&[("/plant/b", 5)]));
        apply_inbound(&variables, &[("/plant/nested/c", 1), ("/plant/a", 3)]).await;
        assert_eq!(next(&mut published).await, owned(&[("/plant/a", 3)]));

        let disabled: InstanceUpdate = serde_json::from_value(json!({"enabled":false})).unwrap();
        let info = manager.save(Some(&instance.id), disabled).await.unwrap();
        assert!(matches!(info.status.state, InstanceState::Disabled));
        apply_inbound(&variables, &[("/plant/a", 4)]).await;
        assert!(
            tokio::time::timeout(Duration::from_millis(300), published.recv())
                .await
                .is_err()
        );
        manager.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn polling_publishes_current_values_every_interval() {
        let Setup {
            manager,
            variables,
            mut published,
            ..
        } = setup().await;
        apply_inbound(&variables, &[("/plant/a", 1), ("/plant/line/b", 2)]).await;
        let config = json!({"variables":["/plant/**"],"mode":{"type":"polling","interval_ms":50},"target":{}});
        manager.save(None, input(config)).await.unwrap();
        let expected = owned(&[("/plant/a", 1), ("/plant/line/b", 2)]);
        assert_eq!(next(&mut published).await, expected);
        assert_eq!(next(&mut published).await, expected);
        apply_inbound(&variables, &[("/plant/a", 7)]).await;
        let mut latest = next(&mut published).await;
        while latest[0].1 != 7 {
            latest = next(&mut published).await;
        }
        assert_eq!(latest, owned(&[("/plant/a", 7), ("/plant/line/b", 2)]));
        manager.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn failed_publishes_are_reported_and_retried_with_latest_values() {
        let Setup {
            manager,
            variables,
            mut published,
            failures,
        } = setup().await;
        apply_inbound(&variables, &[("/plant/a", 1), ("/plant/b", 1)]).await;
        failures.store(1, Ordering::SeqCst);
        let config = json!({"variables":["/plant/*"],"mode":{"type":"subscription"},"target":{}});
        let instance = manager.save(None, input(config)).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        let status = manager.get(&instance.id).await.unwrap().status;
        assert!(matches!(status.state, InstanceState::Error));
        assert_eq!(
            status.last_error.as_deref(),
            Some("destination unavailable")
        );

        apply_inbound(&variables, &[("/plant/a", 2)]).await;
        assert_eq!(
            next(&mut published).await,
            owned(&[("/plant/a", 2), ("/plant/b", 1)])
        );
        assert!(matches!(
            manager.get(&instance.id).await.unwrap().status.state,
            InstanceState::Running
        ));
        manager.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn invalid_configurations_are_rejected_before_saving() {
        let Setup { manager, .. } = setup().await;
        for config in [
            json!({"variables":[],"mode":{"type":"subscription"},"target":{}}),
            json!({"variables":["plant/*"],"mode":{"type":"subscription"},"target":{}}),
            json!({"variables":["/a**"],"mode":{"type":"subscription"},"target":{}}),
            json!({"variables":["/*"],"mode":{"type":"polling","interval_ms":0},"target":{}}),
            json!({"variables":["/*"],"mode":{"type":"polling"},"target":{}}),
            json!({"variables":["/*"],"mode":{"type":"stream"},"target":{}}),
            json!({"variables":["/*"],"mode":{"type":"subscription"},"target":[]}),
            json!({"variables":["/*"],"mode":{"type":"subscription"}}),
        ] {
            assert!(
                matches!(
                    manager.save(None, input(config.clone())).await,
                    Err(InstanceError::Validation(_))
                ),
                "{config}"
            );
        }
        let mut missing =
            input(json!({"variables":["/*"],"mode":{"type":"subscription"},"target":{}}));
        missing.plugin_type = "missing".into();
        assert!(matches!(
            manager.save(None, missing).await,
            Err(InstanceError::Validation(_))
        ));
        assert!(manager.list().await.unwrap().is_empty());
    }
}
