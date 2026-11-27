use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use super::Ingestors;
use crate::http::{
    ApiResponse, ApiResponseEmpty, AppState,
    resources::plugin_instances::{
        InstanceError, InstanceInfo, InstanceInput, InstanceUpdate, PluginManager,
    },
};

pub type IngestorManager = PluginManager<Ingestors>;

#[utoipa::path(get, operation_id = "list_ingestor_plugins", path = "/api/ingestors/plugins", tag = "Ingestors", security(("bearerAuth" = [])),
    responses((status = 200, description = "Compiled-in plugin types", body = [String])))]
pub async fn plugins(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    Json(state.ingestors.plugin_types())
}

#[utoipa::path(get, operation_id = "list_ingestors", path = "/api/ingestors", tag = "Ingestors", security(("bearerAuth" = [])),
    responses((status = 200, description = "Ingestor instances and runtime status", body = ApiResponse<Vec<InstanceInfo>>)))]
pub async fn list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<InstanceInfo>>>, InstanceError> {
    Ok(Json(ApiResponse::success(state.ingestors.list().await?)))
}

#[utoipa::path(get, operation_id = "get_ingestor", path = "/api/ingestors/{id}", tag = "Ingestors", security(("bearerAuth" = [])),
    params(("id" = String, Path, description = "Instance ID")),
    responses((status = 200, description = "Ingestor instance", body = ApiResponse<InstanceInfo>), (status = 404, description = "Not found", body = ApiResponseEmpty)))]
pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<InstanceInfo>>, InstanceError> {
    Ok(Json(ApiResponse::success(state.ingestors.get(&id).await?)))
}

#[utoipa::path(post, operation_id = "create_ingestor", path = "/api/ingestors", tag = "Ingestors", security(("bearerAuth" = [])),
    request_body(content = InstanceInput, example = json!({"name":"Waveform simulator","plugin_type":"simulator","enabled":true,"config":{"interval_ms":1000,"variables":[{"path":"/plant/line[1:10]/ramp[1:200]","waveform":"ramp","min":0,"max":100,"period_ms":10000,"phase_degrees":0,"decimal_places":2},{"path":"/plant/line[1:10]/sine[1:150]","waveform":"sine","min":0,"max":100,"period_ms":5000,"phase_degrees":0,"decimal_places":2},{"path":"/plant/line[1:10]/square[1:100]","waveform":"square","min":0,"max":100,"period_ms":5000,"phase_degrees":90},{"path":"/plant/line[1:10]/random[1:50]","waveform":"random","min":0,"max":100,"decimal_places":2}]}})),
    responses((status = 201, description = "Instance created", body = ApiResponse<InstanceInfo>), (status = 400, description = "Invalid configuration", body = ApiResponseEmpty)))]
pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(input): Json<InstanceInput>,
) -> Result<impl IntoResponse, InstanceError> {
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(
            state.ingestors.save(None, input).await?,
        )),
    ))
}

#[utoipa::path(put, operation_id = "update_ingestor", path = "/api/ingestors/{id}", tag = "Ingestors", security(("bearerAuth" = [])),
    params(("id" = String, Path, description = "Instance ID")), request_body(content = InstanceUpdate, example = json!({"config":{"interval_ms":500}})),
    responses((status = 200, description = "Fields merged; omitted fields remain unchanged; arrays replace existing arrays", body = ApiResponse<InstanceInfo>), (status = 400, description = "Invalid configuration", body = ApiResponseEmpty), (status = 404, description = "Not found", body = ApiResponseEmpty)))]
pub async fn update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(input): Json<InstanceUpdate>,
) -> Result<Json<ApiResponse<InstanceInfo>>, InstanceError> {
    Ok(Json(ApiResponse::success(
        state.ingestors.save(Some(&id), input).await?,
    )))
}

#[utoipa::path(delete, operation_id = "delete_ingestor", path = "/api/ingestors/{id}", tag = "Ingestors", security(("bearerAuth" = [])),
    params(("id" = String, Path, description = "Instance ID")),
    responses((status = 200, description = "Instance deleted; namespace variables are retained", body = ApiResponseEmpty), (status = 404, description = "Not found", body = ApiResponseEmpty)))]
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponseEmpty>, InstanceError> {
    state.ingestors.delete(&id).await?;
    Ok(Json(ApiResponseEmpty::success()))
}

#[cfg(all(test, feature = "simulator"))]
mod tests {
    use super::*;
    use crate::{
        http::resources::plugin_instances::{InstanceState, PluginKind, ensure_schema},
        ingestion::builtin_plugins,
        rtdata::{metrics::Metrics, variable::VariableManager},
    };
    use lirays_scada_proto::namespace::v1::{event::Ev, value::Typed};
    use sea_orm::Database;
    use std::time::Duration;
    use uuid::Uuid;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn simulator_lifecycle_validates_persists_restores_and_stops() {
        let dir = std::env::temp_dir().join(format!("ingestor-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = Database::connect(format!(
            "sqlite://{}?mode=rwc",
            dir.join("static.db").display()
        ))
        .await
        .unwrap();
        ensure_schema(&db, Ingestors::TABLE).await.unwrap();
        let variables = Arc::new(
            VariableManager::new(
                dir.join("rt").to_str().unwrap(),
                Arc::new(Metrics::new(dir.join("metrics"), false, false)),
            )
            .unwrap(),
        );
        let manager =
            IngestorManager::new(db.clone(), variables.clone(), builtin_plugins()).unwrap();
        let mut events = variables.register_listener().await;
        let mut input = InstanceInput {
            name: "Test".into(),
            plugin_type: "simulator".into(),
            enabled: true,
            config: serde_json::json!({"variables":[{"path":"/simulated/value","min":7,"max":7}]}),
        };
        let instance = manager.save(None, input.clone()).await.unwrap();
        let path = "/simulated/value".to_string();
        tokio::time::timeout(Duration::from_secs(3), events.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            variables.values_cache.read().await[&path].val,
            Some(Typed::FloatValue(7.0))
        );
        input.config =
            serde_json::json!({"variables":[{"path":"/simulated/value","min":10,"max":1}]});
        assert!(matches!(
            manager.save(Some(&instance.id), input.clone()).await,
            Err(InstanceError::Validation(_))
        ));
        assert_eq!(
            manager.get(&instance.id).await.unwrap().config,
            serde_json::json!({"variables":[{"path":"/simulated/value","min":7,"max":7}]})
        );
        input.config =
            serde_json::json!({"variables":[{"path":"/simulated/value","min":9,"max":9}]});
        input.enabled = false;
        manager
            .save(Some(&instance.id), input.clone())
            .await
            .unwrap();
        while events.try_recv().is_ok() {}
        assert!(
            tokio::time::timeout(Duration::from_millis(1100), events.recv())
                .await
                .is_err()
        );
        assert!(matches!(
            manager.get(&instance.id).await.unwrap().status.state,
            InstanceState::Disabled
        ));
        input.enabled = true;
        manager.save(Some(&instance.id), input).await.unwrap();
        let batch = tokio::time::timeout(Duration::from_secs(3), events.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(batch.events.iter().any(|e| matches!(&e.ev, Some(Ev::VarValueEv(value)) if value.value.as_ref().unwrap().typed == Some(Typed::FloatValue(9.0)))));
        manager.shutdown().await;
        drop(manager);
        let manager =
            IngestorManager::new(db.clone(), variables.clone(), builtin_plugins()).unwrap();
        manager.restore().await.unwrap();
        assert_eq!(manager.list().await.unwrap().len(), 1);
        assert!(matches!(
            manager.get(&instance.id).await.unwrap().status.state,
            InstanceState::Running
        ));
        manager.delete(&instance.id).await.unwrap();
        while events.try_recv().is_ok() {}
        assert!(
            tokio::time::timeout(Duration::from_millis(1100), events.recv())
                .await
                .is_err()
        );
        assert!(manager.list().await.unwrap().is_empty());
        assert!(variables.values_cache.read().await.contains_key(&path));
        manager.shutdown().await;
        drop(manager);
        drop(variables);
        db.close().await.unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn unavailable_plugins_restore_as_errors_and_can_be_deleted() {
        let dir = std::env::temp_dir().join(format!("ingestor-missing-{}", Uuid::new_v4()));
        let db = Database::connect("sqlite::memory:").await.unwrap();
        ensure_schema(&db, Ingestors::TABLE).await.unwrap();
        let variables = Arc::new(
            VariableManager::new(
                dir.to_str().unwrap(),
                Arc::new(Metrics::new(dir.join("metrics"), false, false)),
            )
            .unwrap(),
        );
        let manager =
            IngestorManager::new(db.clone(), variables.clone(), builtin_plugins()).unwrap();
        let instance = manager
            .save(
                None,
                InstanceInput {
                    name: "Test".into(),
                    plugin_type: "simulator".into(),
                    enabled: true,
                    config: serde_json::json!({"variables":[{"path":"/simulated/value","min":0,"max":1}]}),
                },
            )
            .await
            .unwrap();
        manager.shutdown().await;
        drop(manager);
        let manager = IngestorManager::new(db.clone(), variables.clone(), vec![]).unwrap();
        manager.restore().await.unwrap();
        let status = manager.get(&instance.id).await.unwrap().status;
        assert!(matches!(status.state, InstanceState::Error));
        assert!(status.last_error.unwrap().contains("not compiled"));
        manager.delete(&instance.id).await.unwrap();
        drop(manager);
        drop(variables);
        db.close().await.unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    use crate::{
        http::resources::plugin_instances::{InstanceState, PluginKind, ensure_schema},
        ingestion::{IngestionBatch, Ingestor, IngestorPlugin},
        rtdata::{metrics::Metrics, variable::VariableManager},
    };
    use sea_orm::Database;
    use serde_json::Value;
    use serde_json::json;
    use std::{future::Future, pin::Pin};
    use uuid::Uuid;

    struct TestPlugin;
    struct TestSource;

    impl IngestorPlugin for TestPlugin {
        fn kind(&self) -> &'static str {
            "test-source"
        }
        fn create(&self, _: &str, config: &Value) -> Result<Box<dyn Ingestor>, String> {
            if config["connection"]["host"].as_str().is_none()
                || !config["connection"]["port"]
                    .as_u64()
                    .is_some_and(|port| port > 0)
            {
                return Err("Host and positive port required".into());
            }
            Ok(Box::new(TestSource))
        }
    }

    impl Ingestor for TestSource {
        fn next_batch(
            &mut self,
        ) -> Pin<Box<dyn Future<Output = Result<IngestionBatch, String>> + Send + '_>> {
            Box::pin(std::future::pending())
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn partial_updates_are_plugin_independent_atomic_and_preserve_omitted_fields() {
        let dir = std::env::temp_dir().join(format!("ingestor-merge-{}", Uuid::new_v4()));
        let db = Database::connect("sqlite::memory:").await.unwrap();
        ensure_schema(&db, Ingestors::TABLE).await.unwrap();
        let variables = Arc::new(
            VariableManager::new(
                dir.to_str().unwrap(),
                Arc::new(Metrics::new(dir.join("metrics"), false, false)),
            )
            .unwrap(),
        );
        let manager =
            IngestorManager::new(db.clone(), variables.clone(), vec![Arc::new(TestPlugin)])
                .unwrap();
        let row = manager.save(None, InstanceInput {
            name: "Source".into(), plugin_type: "test-source".into(), enabled: true,
            config: json!({"connection":{"host":"old","port":123,"optional":true},"tags":["a","b"]}),
        }).await.unwrap();
        let patch: InstanceUpdate = serde_json::from_value(
            json!({"config":{"connection":{"host":"new","optional":null},"tags":["c"]}}),
        )
        .unwrap();
        let updated = manager.save(Some(&row.id), patch).await.unwrap();
        assert_eq!(updated.name, "Source");
        assert_eq!(updated.plugin_type, "test-source");
        assert!(updated.enabled);
        assert_eq!(
            updated.config,
            json!({"connection":{"host":"new","port":123},"tags":["c"]})
        );
        let task_id = manager.runtime.lock().await.tasks[&row.id]
            .handle
            .as_ref()
            .unwrap()
            .id();
        manager
            .save(Some(&row.id), InstanceUpdate::default())
            .await
            .unwrap();
        assert_eq!(
            manager.runtime.lock().await.tasks[&row.id]
                .handle
                .as_ref()
                .unwrap()
                .id(),
            task_id
        );
        for document in [
            json!({"config":{"connection":{"port":0}}}),
            json!({"config":{"connection":{"host":null}}}),
            json!({"config":null}),
        ] {
            let patch: InstanceUpdate = serde_json::from_value(document).unwrap();
            assert!(matches!(
                manager.save(Some(&row.id), patch).await,
                Err(InstanceError::Validation(_))
            ));
            assert_eq!(manager.get(&row.id).await.unwrap().config, updated.config);
            assert_eq!(
                manager.runtime.lock().await.tasks[&row.id]
                    .handle
                    .as_ref()
                    .unwrap()
                    .id(),
                task_id
            );
        }
        let name: InstanceUpdate = serde_json::from_value(json!({"name":"Renamed"})).unwrap();
        let config: InstanceUpdate =
            serde_json::from_value(json!({"config":{"connection":{"port":456}}})).unwrap();
        let (first, second) = tokio::join!(
            manager.save(Some(&row.id), name),
            manager.save(Some(&row.id), config)
        );
        first.unwrap();
        second.unwrap();
        let updated = manager.get(&row.id).await.unwrap();
        assert_eq!(updated.name, "Renamed");
        assert_eq!(
            updated.config["connection"],
            json!({"host":"new","port":456})
        );
        let disabled: InstanceUpdate = serde_json::from_value(json!({"enabled":false})).unwrap();
        let updated = manager.save(Some(&row.id), disabled).await.unwrap();
        assert!(matches!(updated.status.state, InstanceState::Disabled));
        assert_eq!(updated.config["tags"], json!(["c"]));
        assert!(matches!(
            manager
                .save(Some("missing"), InstanceUpdate::default())
                .await,
            Err(InstanceError::NotFound)
        ));
        for document in [
            json!({"name":null}),
            json!({"enabled":null}),
            json!({"plugin_type":null}),
            json!({"unknown":true}),
        ] {
            assert!(serde_json::from_value::<InstanceUpdate>(document).is_err());
        }
        manager.shutdown().await;
        drop(manager);
        drop(variables);
        db.close().await.unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }
}
