//! Persistence, validation, and worker lifecycle shared by every kind of plugin instance.

use std::{collections::HashMap, future::Future, sync::Arc};

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbErr, DeriveIden, FromQueryResult, StatementBuilder,
    sea_query::{Alias, ColumnDef, Expr, Order, Query, SelectStatement, Table},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{
    sync::{Mutex, watch},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    db::{create_table_if_missing, large_text},
    http::ApiResponseEmpty,
    rtdata::variable::VariableManager,
};

/// One kind of compiled-in plugin (ingestion, egress, ...) and how its instances run.
pub trait PluginKind: Send + Sync + 'static {
    type Plugin: ?Sized + Send + Sync;
    /// A validated instance, ready to run.
    type Prepared: Send + 'static;

    /// Database table holding this kind's instances.
    const TABLE: &'static str;

    fn plugin_type(plugin: &Self::Plugin) -> &'static str;

    /// Validates the complete configuration without starting background work.
    fn prepare(
        plugin: &Self::Plugin,
        instance_id: &str,
        config: &Value,
    ) -> Result<Self::Prepared, String>;

    /// Runs until `cancel` fires, reporting through `status`. Returning while the
    /// state is still `running` is reported as an unexpected termination.
    fn run(
        prepared: Self::Prepared,
        variables: Arc<VariableManager>,
        cancel: CancellationToken,
        status: watch::Sender<InstanceStatus>,
    ) -> impl Future<Output = ()> + Send + 'static;
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InstanceInput {
    pub name: String,
    pub plugin_type: String,
    pub enabled: bool,
    #[schema(value_type = Object)]
    pub config: Value,
}

#[derive(Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InstanceUpdate {
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
    #[schema(value_type = Option<Object>)]
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
pub enum InstanceState {
    Running,
    Disabled,
    Stopped,
    Error,
}

#[derive(Clone, Serialize, ToSchema)]
pub struct InstanceStatus {
    pub state: InstanceState,
    pub last_error: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct InstanceInfo {
    pub id: String,
    pub name: String,
    pub plugin_type: String,
    pub enabled: bool,
    #[schema(value_type = Object)]
    pub config: Value,
    pub status: InstanceStatus,
}

#[derive(Debug)]
pub enum InstanceError {
    Validation(String),
    NotFound,
    Database(DbErr),
    Stopping,
}

impl From<DbErr> for InstanceError {
    fn from(error: DbErr) -> Self {
        Self::Database(error)
    }
}

impl IntoResponse for InstanceError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::Validation(message) => (StatusCode::BAD_REQUEST, message),
            Self::NotFound => (StatusCode::NOT_FOUND, "Instance not found".into()),
            Self::Database(error) => {
                log::error!("Plugin instance database error: {error}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Plugin instance database error".into(),
                )
            }
            Self::Stopping => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Plugin instances are shutting down".into(),
            ),
        };
        (status, Json(ApiResponseEmpty::error(message))).into_response()
    }
}

#[derive(DeriveIden)]
enum Instance {
    Id,
    Name,
    PluginType,
    Enabled,
    ConfigJson,
}

pub async fn ensure_schema(db: &DatabaseConnection, table: &str) -> Result<(), DbErr> {
    let backend = db.get_database_backend();
    create_table_if_missing(
        db,
        Table::create()
            .table(Alias::new(table))
            .col(
                ColumnDef::new(Instance::Id)
                    .string()
                    .not_null()
                    .primary_key(),
            )
            .col(ColumnDef::new(Instance::Name).string().not_null())
            .col(ColumnDef::new(Instance::PluginType).string().not_null())
            .col(ColumnDef::new(Instance::Enabled).boolean().not_null())
            .col(ColumnDef::new_with_type(Instance::ConfigJson, large_text(backend)).not_null())
            .to_owned(),
    )
    .await
}

#[derive(FromQueryResult)]
struct Row {
    id: String,
    name: String,
    plugin_type: String,
    enabled: bool,
    config_json: String,
}

pub(super) struct InstanceTask {
    cancel: CancellationToken,
    pub(super) handle: Option<JoinHandle<()>>,
    status: watch::Receiver<InstanceStatus>,
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
pub(super) struct Runtime {
    pub(super) tasks: HashMap<String, InstanceTask>,
    stopping: bool,
}

pub struct PluginManager<K: PluginKind> {
    db: DatabaseConnection,
    variables: Arc<VariableManager>,
    plugins: HashMap<&'static str, Arc<K::Plugin>>,
    pub(super) runtime: Mutex<Runtime>,
}

impl<K: PluginKind> PluginManager<K> {
    pub fn new(
        db: DatabaseConnection,
        variables: Arc<VariableManager>,
        plugins: Vec<Arc<K::Plugin>>,
    ) -> Result<Self, String> {
        let mut registry = HashMap::new();
        for plugin in plugins {
            if registry.insert(K::plugin_type(&plugin), plugin).is_some() {
                return Err(format!("Duplicate plugin registration in {}", K::TABLE));
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

    fn prepare(&self, id: &str, plugin_type: &str, config: &Value) -> Result<K::Prepared, String> {
        let plugin = self
            .plugins
            .get(plugin_type)
            .ok_or_else(|| format!("Plugin is not compiled into this binary: {plugin_type}"))?;
        K::prepare(plugin, id, config)
    }

    fn start_task(&self, prepared: Result<K::Prepared, String>) -> InstanceTask {
        let cancel = CancellationToken::new();
        let (status_tx, status) = watch::channel(InstanceStatus {
            state: InstanceState::Running,
            last_error: None,
        });
        let handle = match prepared {
            Err(error) => {
                status_tx.send_replace(InstanceStatus {
                    state: InstanceState::Error,
                    last_error: Some(error),
                });
                None
            }
            Ok(prepared) => Some(tokio::spawn(K::run(
                prepared,
                self.variables.clone(),
                cancel.clone(),
                status_tx,
            ))),
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

    fn select() -> SelectStatement {
        Query::select()
            .columns([
                Instance::Id,
                Instance::Name,
                Instance::PluginType,
                Instance::Enabled,
                Instance::ConfigJson,
            ])
            .from(Alias::new(K::TABLE))
            .to_owned()
    }

    fn id_is(id: impl Into<Expr>) -> Expr {
        use sea_orm::sea_query::ExprTrait;
        Expr::col(Instance::Id).eq(id)
    }

    async fn find(&self, id: &str) -> Result<Option<Row>, DbErr> {
        let query = Self::select().and_where(Self::id_is(id)).to_owned();
        Row::find_by_statement(self.db.get_database_backend().build(&query))
            .one(&self.db)
            .await
    }

    async fn find_all(&self) -> Result<Vec<Row>, DbErr> {
        let query = Self::select().order_by(Instance::Id, Order::Asc).to_owned();
        Row::find_by_statement(self.db.get_database_backend().build(&query))
            .all(&self.db)
            .await
    }

    async fn execute(&self, statement: &impl StatementBuilder) -> Result<u64, DbErr> {
        Ok(self.db.execute(statement).await?.rows_affected())
    }

    /// Restore enabled instances; unavailable plugins remain persisted with an error status.
    pub async fn restore(&self) -> Result<(), DbErr> {
        let mut runtime = self.runtime.lock().await;
        if runtime.stopping {
            return Ok(());
        }
        for row in self.find_all().await? {
            if row.enabled && !runtime.tasks.contains_key(&row.id) {
                let prepared = serde_json::from_str(&row.config_json)
                    .map_err(|e| e.to_string())
                    .and_then(|config| self.prepare(&row.id, &row.plugin_type, &config));
                runtime.tasks.insert(row.id, self.start_task(prepared));
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

    fn info(row: Row, runtime: &Runtime) -> Result<InstanceInfo, InstanceError> {
        let status = match runtime.tasks.get(&row.id) {
            Some(task) => {
                let mut status = task.status.borrow().clone();
                if matches!(status.state, InstanceState::Running)
                    && task.handle.as_ref().is_some_and(|h| h.is_finished())
                {
                    status = InstanceStatus {
                        state: InstanceState::Error,
                        last_error: Some("Instance task terminated unexpectedly".into()),
                    };
                }
                status
            }
            None => InstanceStatus {
                state: if row.enabled {
                    InstanceState::Stopped
                } else {
                    InstanceState::Disabled
                },
                last_error: None,
            },
        };
        Ok(InstanceInfo {
            id: row.id,
            name: row.name,
            plugin_type: row.plugin_type,
            enabled: row.enabled,
            config: serde_json::from_str(&row.config_json)
                .map_err(|e| InstanceError::Database(DbErr::Custom(e.to_string())))?,
            status,
        })
    }

    pub async fn list(&self) -> Result<Vec<InstanceInfo>, InstanceError> {
        let runtime = self.runtime.lock().await;
        self.find_all()
            .await?
            .into_iter()
            .map(|row| Self::info(row, &runtime))
            .collect()
    }

    pub async fn get(&self, id: &str) -> Result<InstanceInfo, InstanceError> {
        let runtime = self.runtime.lock().await;
        let row = self.find(id).await?.ok_or(InstanceError::NotFound)?;
        Self::info(row, &runtime)
    }

    /// Merge updates under the lifecycle lock, then validate the complete plugin configuration.
    pub async fn save<T: Serialize>(
        &self,
        id: Option<&str>,
        input: T,
    ) -> Result<InstanceInfo, InstanceError> {
        let mut runtime = self.runtime.lock().await;
        if runtime.stopping {
            return Err(InstanceError::Stopping);
        }
        let mut document =
            serde_json::to_value(input).map_err(|e| InstanceError::Validation(e.to_string()))?;
        let instance_id = match id {
            Some(id) => {
                let existing = self.find(id).await?.ok_or(InstanceError::NotFound)?;
                if document.as_object().is_some_and(|fields| fields.is_empty()) {
                    return Self::info(existing, &runtime);
                }
                let config: Value = serde_json::from_str(&existing.config_json)
                    .map_err(|e| InstanceError::Database(DbErr::Custom(e.to_string())))?;
                let mut merged = serde_json::json!({
                    "name": existing.name, "plugin_type": existing.plugin_type,
                    "enabled": existing.enabled, "config": config,
                });
                merge_config(&mut merged, document);
                document = merged;
                existing.id
            }
            None => Uuid::new_v4().to_string(),
        };
        let input: InstanceInput = serde_json::from_value(document)
            .map_err(|e| InstanceError::Validation(e.to_string()))?;
        if input.name.trim().is_empty() {
            return Err(InstanceError::Validation("Name cannot be blank".into()));
        }
        let prepared = self
            .prepare(&instance_id, &input.plugin_type, &input.config)
            .map_err(InstanceError::Validation)?;
        let row = Row {
            id: instance_id,
            name: input.name.trim().into(),
            plugin_type: input.plugin_type,
            enabled: input.enabled,
            config_json: input.config.to_string(),
        };
        let values = [
            (Instance::Name, row.name.clone().into()),
            (Instance::PluginType, row.plugin_type.clone().into()),
            (Instance::Enabled, row.enabled.into()),
            (Instance::ConfigJson, row.config_json.clone().into()),
        ];
        if id.is_some() {
            let update = Query::update()
                .table(Alias::new(K::TABLE))
                .values(values)
                .and_where(Self::id_is(row.id.clone()))
                .to_owned();
            self.execute(&update).await?;
        } else {
            let (columns, values): (Vec<_>, Vec<_>) = values.into_iter().unzip();
            let insert = Query::insert()
                .into_table(Alias::new(K::TABLE))
                .columns(columns.into_iter().chain([Instance::Id]))
                .values_panic(values.into_iter().chain([row.id.clone().into()]))
                .to_owned();
            self.execute(&insert).await?;
        }
        Self::stop_task(&mut runtime, &row.id).await;
        if row.enabled {
            runtime
                .tasks
                .insert(row.id.clone(), self.start_task(Ok(prepared)));
        }
        Self::info(row, &runtime)
    }

    pub async fn delete(&self, id: &str) -> Result<(), InstanceError> {
        let mut runtime = self.runtime.lock().await;
        if runtime.stopping {
            return Err(InstanceError::Stopping);
        }
        let delete = Query::delete()
            .from_table(Alias::new(K::TABLE))
            .and_where(Self::id_is(id))
            .to_owned();
        if self.execute(&delete).await? == 0 {
            return Err(InstanceError::NotFound);
        }
        Self::stop_task(&mut runtime, id).await;
        Ok(())
    }
}
