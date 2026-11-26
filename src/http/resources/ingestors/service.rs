use std::{collections::HashMap, sync::Arc};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{
    sync::{Mutex, watch},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use utoipa::ToSchema;
use uuid::Uuid;

use super::model::{self, Entity};
use crate::{
    http::{ApiResponse, ApiResponseEmpty, AppState},
    ingestion::{Ingestor, IngestorPlugin},
    rtdata::variable::VariableManager,
};

#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct IngestorInput {
    pub name: String,
    #[schema(example = "simulator")]
    pub plugin_type: String,
    pub enabled: bool,
    #[schema(value_type = Object, example = json!({"interval_ms":100,"variables":[{"path":"/plant/random","waveform":"random","min":0,"max":100,"decimal_places":2},{"path":"/plant/sine","waveform":"sine","min":0,"max":100,"period_ms":5000,"phase_degrees":0,"decimal_places":2},{"path":"/plant/square","waveform":"square","min":0,"max":100,"period_ms":5000,"phase_degrees":90},{"path":"/plant/ramp","waveform":"ramp","min":0,"max":100,"period_ms":10000,"phase_degrees":0,"decimal_places":2}]}))]
    pub config: Value,
}

#[derive(Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct IngestorUpdate {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_field"
    )]
    pub name: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_field"
    )]
    pub plugin_type: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_field"
    )]
    pub enabled: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_field"
    )]
    #[schema(value_type = Option<Object>, example = json!({"interval_ms":500}))]
    pub config: Option<Value>,
}

fn present_field<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

// Objects merge recursively; null removes a key, while arrays and scalars replace it.
fn merge_config(target: &mut Value, patch: Value) {
    match patch {
        Value::Object(fields) => {
            if !target.is_object() {
                *target = serde_json::json!({});
            }
            let object = target.as_object_mut().unwrap();
            for (key, value) in fields {
                if value.is_null() {
                    object.remove(&key);
                } else {
                    merge_config(object.entry(key).or_insert(Value::Null), value);
                }
            }
        }
        value => *target = value,
    }
}

#[derive(Clone, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IngestorState {
    Running,
    Disabled,
    Stopped,
    Error,
}

#[derive(Clone, Serialize, ToSchema)]
pub struct IngestorStatus {
    pub state: IngestorState,
    pub last_error: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct IngestorInfo {
    pub id: String,
    pub name: String,
    pub plugin_type: String,
    pub enabled: bool,
    #[schema(value_type = Object)]
    pub config: Value,
    pub status: IngestorStatus,
}

#[derive(Debug)]
pub enum IngestorError {
    Validation(String),
    NotFound,
    Database(sea_orm::DbErr),
    Stopping,
}

impl From<sea_orm::DbErr> for IngestorError {
    fn from(error: sea_orm::DbErr) -> Self {
        Self::Database(error)
    }
}

impl IntoResponse for IngestorError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::Validation(message) => (StatusCode::BAD_REQUEST, message),
            Self::NotFound => (StatusCode::NOT_FOUND, "Ingestor not found".into()),
            Self::Database(error) => {
                log::error!("Ingestor database error: {error}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Ingestor database error".into(),
                )
            }
            Self::Stopping => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Ingestors are shutting down".into(),
            ),
        };
        (status, Json(ApiResponseEmpty::error(message))).into_response()
    }
}

struct InstanceTask {
    cancel: CancellationToken,
    handle: Option<JoinHandle<()>>,
    status: watch::Receiver<IngestorStatus>,
}

impl Drop for InstanceTask {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Some(handle) = &self.handle {
            handle.abort();
        }
    }
}

#[derive(Default)]
struct Runtime {
    tasks: HashMap<String, InstanceTask>,
    stopping: bool,
}

pub struct IngestorManager {
    db: DatabaseConnection,
    variables: Arc<VariableManager>,
    plugins: HashMap<&'static str, Arc<dyn IngestorPlugin>>,
    runtime: Mutex<Runtime>,
}

impl IngestorManager {
    pub fn new(
        db: DatabaseConnection,
        variables: Arc<VariableManager>,
        plugins: Vec<Arc<dyn IngestorPlugin>>,
    ) -> Result<Self, String> {
        let mut registry = HashMap::new();
        for plugin in plugins {
            if registry.insert(plugin.kind(), plugin).is_some() {
                return Err("Duplicate ingestor plugin registration".into());
            }
        }
        Ok(Self {
            db,
            variables,
            plugins: registry,
            runtime: Mutex::new(Runtime::default()),
        })
    }

    pub fn plugin_types(&self) -> Vec<String> {
        let mut types: Vec<_> = self.plugins.keys().map(|kind| kind.to_string()).collect();
        types.sort();
        types
    }

    fn build(&self, id: &str, input: &IngestorInput) -> Result<Box<dyn Ingestor>, IngestorError> {
        if input.name.trim().is_empty() {
            return Err(IngestorError::Validation("Name cannot be blank".into()));
        }
        self.plugins
            .get(input.plugin_type.as_str())
            .ok_or_else(|| {
                IngestorError::Validation(format!(
                    "Plugin is not compiled into this binary: {}",
                    input.plugin_type
                ))
            })?
            .create(id, &input.config)
            .map_err(IngestorError::Validation)
    }

    fn start_task(&self, source: Result<Box<dyn Ingestor>, String>) -> InstanceTask {
        let cancel = CancellationToken::new();
        let (status_tx, status) = watch::channel(IngestorStatus {
            state: IngestorState::Running,
            last_error: None,
        });
        let handle = match source {
            Err(error) => {
                status_tx.send_replace(IngestorStatus {
                    state: IngestorState::Error,
                    last_error: Some(error),
                });
                None
            }
            Ok(mut source) => {
                let token = cancel.clone();
                let variables = self.variables.clone();
                Some(tokio::spawn(async move {
                    loop {
                        let next = tokio::select! {
                            biased;
                            _ = token.cancelled() => break,
                            next = source.next_batch() => next,
                        };
                        let result = match next {
                            Ok(batch) => variables.ingest(batch).await,
                            Err(error) => Err(error),
                        };
                        if let Err(error) = result {
                            log::warn!("Ingestor stopped: {error}");
                            status_tx.send_replace(IngestorStatus {
                                state: IngestorState::Error,
                                last_error: Some(error),
                            });
                            return;
                        }
                    }
                    status_tx.send_replace(IngestorStatus {
                        state: IngestorState::Stopped,
                        last_error: None,
                    });
                }))
            }
        };
        InstanceTask {
            cancel,
            handle,
            status,
        }
    }

    async fn stop_task(runtime: &mut Runtime, id: &str) {
        if let Some(mut task) = runtime.tasks.remove(id) {
            task.cancel.cancel();
            if let Some(handle) = task.handle.take() {
                let _ = handle.await;
            }
        }
    }

    /// Restore enabled instances; unavailable plugins remain persisted with an error status.
    pub async fn restore(&self) -> Result<(), sea_orm::DbErr> {
        let mut runtime = self.runtime.lock().await;
        if runtime.stopping {
            return Ok(());
        }
        for row in Entity::find().all(&self.db).await? {
            if row.enabled && !runtime.tasks.contains_key(&row.id) {
                let source = serde_json::from_str(&row.config_json)
                    .map_err(|e| e.to_string())
                    .and_then(|config| {
                        self.plugins
                            .get(row.plugin_type.as_str())
                            .ok_or_else(|| {
                                format!(
                                    "Plugin is not compiled into this binary: {}",
                                    row.plugin_type
                                )
                            })?
                            .create(&row.id, &config)
                    });
                runtime.tasks.insert(row.id, self.start_task(source));
            }
        }
        Ok(())
    }

    pub async fn shutdown(&self) {
        let mut runtime = self.runtime.lock().await;
        runtime.stopping = true;
        let ids: Vec<_> = runtime.tasks.keys().cloned().collect();
        for id in ids {
            Self::stop_task(&mut runtime, &id).await;
        }
    }

    fn info(row: model::Model, runtime: &Runtime) -> Result<IngestorInfo, IngestorError> {
        let status = match runtime.tasks.get(&row.id) {
            Some(task) => {
                let mut status = task.status.borrow().clone();
                if matches!(status.state, IngestorState::Running)
                    && task.handle.as_ref().is_some_and(|h| h.is_finished())
                {
                    status = IngestorStatus {
                        state: IngestorState::Error,
                        last_error: Some("Ingestor task terminated unexpectedly".into()),
                    };
                }
                status
            }
            None => IngestorStatus {
                state: if row.enabled {
                    IngestorState::Stopped
                } else {
                    IngestorState::Disabled
                },
                last_error: None,
            },
        };
        Ok(IngestorInfo {
            id: row.id,
            name: row.name,
            plugin_type: row.plugin_type,
            enabled: row.enabled,
            config: serde_json::from_str(&row.config_json)
                .map_err(|e| IngestorError::Database(sea_orm::DbErr::Custom(e.to_string())))?,
            status,
        })
    }

    pub async fn list(&self) -> Result<Vec<IngestorInfo>, IngestorError> {
        let runtime = self.runtime.lock().await;
        Entity::find()
            .order_by_asc(model::Column::Id)
            .all(&self.db)
            .await?
            .into_iter()
            .map(|row| Self::info(row, &runtime))
            .collect()
    }

    pub async fn get(&self, id: &str) -> Result<IngestorInfo, IngestorError> {
        let runtime = self.runtime.lock().await;
        let row = Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(IngestorError::NotFound)?;
        Self::info(row, &runtime)
    }

    /// Merge updates under the lifecycle lock, then validate the complete plugin configuration.
    pub async fn save<T: Serialize>(
        &self,
        id: Option<&str>,
        input: T,
    ) -> Result<IngestorInfo, IngestorError> {
        let mut runtime = self.runtime.lock().await;
        if runtime.stopping {
            return Err(IngestorError::Stopping);
        }
        let mut document =
            serde_json::to_value(input).map_err(|e| IngestorError::Validation(e.to_string()))?;
        let mut active = if let Some(id) = id {
            let existing = Entity::find_by_id(id)
                .one(&self.db)
                .await?
                .ok_or(IngestorError::NotFound)?;
            if document.as_object().is_some_and(|fields| fields.is_empty()) {
                return Self::info(existing, &runtime);
            }
            let config: Value = serde_json::from_str(&existing.config_json)
                .map_err(|e| IngestorError::Database(sea_orm::DbErr::Custom(e.to_string())))?;
            let mut merged = serde_json::json!({
                "name": existing.name, "plugin_type": existing.plugin_type,
                "enabled": existing.enabled, "config": config,
            });
            merge_config(&mut merged, document);
            document = merged;
            existing.into()
        } else {
            model::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                ..Default::default()
            }
        };
        let input: IngestorInput = serde_json::from_value(document)
            .map_err(|e| IngestorError::Validation(e.to_string()))?;
        let instance_id = active.id.clone().unwrap();
        let source = self.build(&instance_id, &input)?;
        active.name = Set(input.name.trim().into());
        active.plugin_type = Set(input.plugin_type);
        active.enabled = Set(input.enabled);
        active.config_json = Set(input.config.to_string());
        let row = if id.is_some() {
            active.update(&self.db).await?
        } else {
            active.insert(&self.db).await?
        };
        Self::stop_task(&mut runtime, &instance_id).await;
        if row.enabled {
            runtime
                .tasks
                .insert(instance_id, self.start_task(Ok(source)));
        }
        Self::info(row, &runtime)
    }

    pub async fn delete(&self, id: &str) -> Result<(), IngestorError> {
        let mut runtime = self.runtime.lock().await;
        if runtime.stopping {
            return Err(IngestorError::Stopping);
        }
        let deleted = Entity::delete_by_id(id).exec(&self.db).await?;
        if deleted.rows_affected == 0 {
            return Err(IngestorError::NotFound);
        }
        Self::stop_task(&mut runtime, id).await;
        Ok(())
    }
}

#[utoipa::path(get, path = "/api/ingestors/plugins", tag = "Ingestors", security(("bearerAuth" = [])),
    responses((status = 200, description = "Compiled-in plugin types", body = [String])))]
pub async fn plugins(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    Json(state.ingestors.plugin_types())
}

#[utoipa::path(get, path = "/api/ingestors", tag = "Ingestors", security(("bearerAuth" = [])),
    responses((status = 200, description = "Ingestor instances and runtime status", body = ApiResponseIngestors)))]
pub async fn list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<IngestorInfo>>>, IngestorError> {
    Ok(Json(ApiResponse::success(state.ingestors.list().await?)))
}

#[utoipa::path(get, path = "/api/ingestors/{id}", tag = "Ingestors", security(("bearerAuth" = [])),
    params(("id" = String, Path, description = "Instance ID")),
    responses((status = 200, description = "Ingestor instance", body = ApiResponseIngestor), (status = 404, description = "Not found", body = ApiResponseEmpty)))]
pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<IngestorInfo>>, IngestorError> {
    Ok(Json(ApiResponse::success(state.ingestors.get(&id).await?)))
}

#[utoipa::path(post, path = "/api/ingestors", tag = "Ingestors", security(("bearerAuth" = [])),
    request_body(content = IngestorInput, example = json!({"name":"Waveform simulator","plugin_type":"simulator","enabled":true,"config":{"interval_ms":100,"variables":[{"path":"/plant/random","waveform":"random","min":0,"max":100,"decimal_places":2},{"path":"/plant/sine","waveform":"sine","min":0,"max":100,"period_ms":5000,"phase_degrees":0,"decimal_places":2},{"path":"/plant/square","waveform":"square","min":0,"max":100,"period_ms":5000,"phase_degrees":90},{"path":"/plant/ramp","waveform":"ramp","min":0,"max":100,"period_ms":10000,"phase_degrees":0,"decimal_places":2}]}})),
    responses((status = 201, description = "Instance created", body = ApiResponseIngestor), (status = 400, description = "Invalid configuration", body = ApiResponseEmpty)))]
pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(input): Json<IngestorInput>,
) -> Result<impl IntoResponse, IngestorError> {
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(
            state.ingestors.save(None, input).await?,
        )),
    ))
}

#[utoipa::path(put, path = "/api/ingestors/{id}", tag = "Ingestors", security(("bearerAuth" = [])),
    params(("id" = String, Path, description = "Instance ID")), request_body(content = IngestorUpdate, example = json!({"config":{"interval_ms":500}})),
    responses((status = 200, description = "Fields merged; omitted fields remain unchanged; arrays replace existing arrays", body = ApiResponseIngestor), (status = 400, description = "Invalid configuration", body = ApiResponseEmpty), (status = 404, description = "Not found", body = ApiResponseEmpty)))]
pub async fn update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(input): Json<IngestorUpdate>,
) -> Result<Json<ApiResponse<IngestorInfo>>, IngestorError> {
    Ok(Json(ApiResponse::success(
        state.ingestors.save(Some(&id), input).await?,
    )))
}

#[utoipa::path(delete, path = "/api/ingestors/{id}", tag = "Ingestors", security(("bearerAuth" = [])),
    params(("id" = String, Path, description = "Instance ID")),
    responses((status = 200, description = "Instance deleted; namespace variables are retained", body = ApiResponseEmpty), (status = 404, description = "Not found", body = ApiResponseEmpty)))]
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponseEmpty>, IngestorError> {
    state.ingestors.delete(&id).await?;
    Ok(Json(ApiResponseEmpty::success()))
}

#[cfg(all(test, feature = "simulator"))]
mod tests {
    use super::*;
    use crate::{ingestion::builtin_plugins, rtdata::metrics::Metrics};
    use lirays_scada_proto::namespace::v1::{event::Ev, value::Typed};
    use sea_orm::Database;
    use std::time::Duration;

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
        super::super::ensure_schema(&db).await.unwrap();
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
        let mut input = IngestorInput {
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
            Err(IngestorError::Validation(_))
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
            IngestorState::Disabled
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
            IngestorState::Running
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
        super::super::ensure_schema(&db).await.unwrap();
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
                IngestorInput {
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
        assert!(matches!(status.state, IngestorState::Error));
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
    use crate::{ingestion::IngestionBatch, rtdata::metrics::Metrics};
    use sea_orm::Database;
    use serde_json::json;
    use std::{future::Future, pin::Pin};

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
        super::super::ensure_schema(&db).await.unwrap();
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
        let row = manager.save(None, IngestorInput {
            name: "Source".into(), plugin_type: "test-source".into(), enabled: true,
            config: json!({"connection":{"host":"old","port":123,"optional":true},"tags":["a","b"]}),
        }).await.unwrap();
        let patch: IngestorUpdate = serde_json::from_value(
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
            .save(Some(&row.id), IngestorUpdate::default())
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
            let patch: IngestorUpdate = serde_json::from_value(document).unwrap();
            assert!(matches!(
                manager.save(Some(&row.id), patch).await,
                Err(IngestorError::Validation(_))
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
        let name: IngestorUpdate = serde_json::from_value(json!({"name":"Renamed"})).unwrap();
        let config: IngestorUpdate =
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
        let disabled: IngestorUpdate = serde_json::from_value(json!({"enabled":false})).unwrap();
        let updated = manager.save(Some(&row.id), disabled).await.unwrap();
        assert!(matches!(updated.status.state, IngestorState::Disabled));
        assert_eq!(updated.config["tags"], json!(["c"]));
        assert!(matches!(
            manager
                .save(Some("missing"), IngestorUpdate::default())
                .await,
            Err(IngestorError::NotFound)
        ));
        for document in [
            json!({"name":null}),
            json!({"enabled":null}),
            json!({"plugin_type":null}),
            json!({"unknown":true}),
        ] {
            assert!(serde_json::from_value::<IngestorUpdate>(document).is_err());
        }
        manager.shutdown().await;
        drop(manager);
        drop(variables);
        db.close().await.unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }
}
