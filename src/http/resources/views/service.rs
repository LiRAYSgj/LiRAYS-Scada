use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set, SqlErr, Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::model::{self, Entity as ViewEntity};
use crate::http::{ApiResponse, ApiResponseEmpty, AppState};

const EMPTY_GRAPH_JSON: &str =
    r#"{"version":1,"nodes":[],"edges":[],"viewport":{"x":0,"y":0,"zoom":1}}"#;

fn current_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

fn default_canvas_json() -> String {
    EMPTY_GRAPH_JSON.to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct View {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_entry_point: bool,
    pub canvas_json: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ViewInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub is_entry_point: Option<bool>,
    #[serde(default = "default_canvas_json")]
    pub canvas_json: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ViewPage {
    pub items: Vec<View>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub sort_by: ViewSortBy,
    pub sort_direction: SortDirection,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ViewSortBy {
    Name,
    UpdatedAt,
    IsEntryPoint,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Deserialize, Debug, Clone, ToSchema)]
pub struct ViewListQuery {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
    #[serde(default = "default_sort_by")]
    pub sort_by: ViewSortBy,
    #[serde(default = "default_sort_direction")]
    pub sort_direction: SortDirection,
    #[serde(default)]
    pub search: Option<String>,
}

fn default_page() -> u64 {
    1
}

fn default_page_size() -> u64 {
    10
}

fn default_sort_by() -> ViewSortBy {
    ViewSortBy::UpdatedAt
}

fn default_sort_direction() -> SortDirection {
    SortDirection::Desc
}

#[derive(Debug)]
pub enum ViewManagerError {
    NotFound,
    DuplicateName(String),
    Validation(String),
    Database(sea_orm::DbErr),
}

impl std::fmt::Display for ViewManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "View not found"),
            Self::DuplicateName(name) => write!(f, "View name \"{name}\" already exists"),
            Self::Validation(msg) => write!(f, "{msg}"),
            Self::Database(err) => write!(f, "Database error: {err}"),
        }
    }
}

impl From<sea_orm::DbErr> for ViewManagerError {
    fn from(value: sea_orm::DbErr) -> Self {
        Self::Database(value)
    }
}

impl std::error::Error for ViewManagerError {}

impl From<model::Model> for View {
    fn from(model: model::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            description: model.description,
            is_entry_point: model.is_entry_point,
            canvas_json: model.canvas_json,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

fn validate_name(name: &str) -> Result<String, ViewManagerError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(ViewManagerError::Validation(
            "View name cannot be blank".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

fn validate_description(description: &str) -> String {
    description.trim().to_string()
}

fn map_view_name_unique_error(error: sea_orm::DbErr, name: &str) -> ViewManagerError {
    if let Some(SqlErr::UniqueConstraintViolation(message)) = error.sql_err() {
        if message.to_ascii_lowercase().contains("views.name") {
            return ViewManagerError::DuplicateName(name.to_string());
        }
    }
    ViewManagerError::Database(error)
}

fn validate_canvas_json(canvas_json: &str) -> Result<(), ViewManagerError> {
    serde_json::from_str::<serde_json::Value>(canvas_json)
        .map_err(|_| ViewManagerError::Validation("canvas_json must be valid JSON".to_string()))?;
    Ok(())
}

pub struct ViewManager {
    db: DatabaseConnection,
}

impl ViewManager {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn ensure_initialized(&self) -> Result<(), sea_orm::DbErr> {
        self.db
            .execute(Statement::from_string(
                self.db.get_database_backend(),
                "CREATE TABLE IF NOT EXISTS views (
                    id TEXT PRIMARY KEY NOT NULL,
                    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
                    description TEXT NOT NULL DEFAULT '',
                    is_entry_point BOOLEAN NOT NULL DEFAULT 0,
                    canvas_json TEXT NOT NULL DEFAULT '{}',
                    created_at BIGINT NOT NULL,
                    updated_at BIGINT NOT NULL
                )"
                .to_string(),
            ))
            .await?;

        let count = ViewEntity::find().count(&self.db).await?;
        if count == 0 {
            let now = current_ts();
            let root = model::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                name: Set("Root View".to_string()),
                description: Set(String::new()),
                is_entry_point: Set(true),
                canvas_json: Set(default_canvas_json()),
                created_at: Set(now),
                updated_at: Set(now),
            };
            root.insert(&self.db).await?;
        }
        Ok(())
    }

    pub async fn list_views_page(&self, query: ViewListQuery) -> Result<ViewPage, sea_orm::DbErr> {
        let page_size = query.page_size.clamp(1, 100);
        let page_number = query.page.max(1);
        let page_index = page_number.saturating_sub(1);

        let mut select = match (query.sort_by, query.sort_direction) {
            (ViewSortBy::Name, SortDirection::Asc) => ViewEntity::find()
                .order_by_asc(model::Column::Name)
                .order_by_asc(model::Column::Id),
            (ViewSortBy::Name, SortDirection::Desc) => ViewEntity::find()
                .order_by_desc(model::Column::Name)
                .order_by_asc(model::Column::Id),
            (ViewSortBy::UpdatedAt, SortDirection::Asc) => ViewEntity::find()
                .order_by_asc(model::Column::UpdatedAt)
                .order_by_asc(model::Column::Id),
            (ViewSortBy::UpdatedAt, SortDirection::Desc) => ViewEntity::find()
                .order_by_desc(model::Column::UpdatedAt)
                .order_by_asc(model::Column::Id),
            (ViewSortBy::IsEntryPoint, SortDirection::Asc) => ViewEntity::find()
                .order_by_asc(model::Column::IsEntryPoint)
                .order_by_desc(model::Column::UpdatedAt)
                .order_by_asc(model::Column::Id),
            (ViewSortBy::IsEntryPoint, SortDirection::Desc) => ViewEntity::find()
                .order_by_desc(model::Column::IsEntryPoint)
                .order_by_desc(model::Column::UpdatedAt)
                .order_by_asc(model::Column::Id),
        };

        if let Some(search) = query.search.as_deref().map(str::trim) {
            if !search.is_empty() {
                select = select.filter(
                    Condition::any()
                        .add(model::Column::Name.contains(search))
                        .add(model::Column::Description.contains(search)),
                );
            }
        }

        let paginator = select.paginate(&self.db, page_size);
        let total = paginator.num_items().await?;
        let models = paginator.fetch_page(page_index).await?;

        Ok(ViewPage {
            items: models.into_iter().map(Into::into).collect(),
            total,
            page: page_number,
            page_size,
            sort_by: query.sort_by,
            sort_direction: query.sort_direction,
        })
    }

    pub async fn get_view(&self, id: &str) -> Result<Option<View>, sea_orm::DbErr> {
        let model = ViewEntity::find_by_id(id.to_string()).one(&self.db).await?;
        Ok(model.map(Into::into))
    }

    pub async fn get_entry_point(&self) -> Result<Option<View>, sea_orm::DbErr> {
        let model = ViewEntity::find()
            .filter(model::Column::IsEntryPoint.eq(true))
            .order_by_desc(model::Column::UpdatedAt)
            .one(&self.db)
            .await?;
        Ok(model.map(Into::into))
    }

    pub async fn create_view(&self, input: ViewInput) -> Result<View, ViewManagerError> {
        let name = validate_name(&input.name)?;
        let description = validate_description(&input.description);
        validate_canvas_json(&input.canvas_json)?;
        let now = current_ts();
        let should_be_entry_point = input.is_entry_point.unwrap_or(false);
        let name_for_error = name.clone();

        let txn = self.db.begin().await?;

        if should_be_entry_point {
            let current_entry_points = ViewEntity::find()
                .filter(model::Column::IsEntryPoint.eq(true))
                .all(&txn)
                .await?;
            for entry in current_entry_points {
                let mut active: model::ActiveModel = entry.into();
                active.is_entry_point = Set(false);
                active.updated_at = Set(now);
                active.update(&txn).await?;
            }
        }

        let active = model::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            name: Set(name),
            description: Set(description),
            is_entry_point: Set(should_be_entry_point),
            canvas_json: Set(input.canvas_json),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let created = active
            .insert(&txn)
            .await
            .map_err(|error| map_view_name_unique_error(error, &name_for_error))?;

        txn.commit().await?;
        Ok(created.into())
    }

    pub async fn update_view(&self, id: &str, input: ViewInput) -> Result<View, ViewManagerError> {
        let name = validate_name(&input.name)?;
        let description = validate_description(&input.description);
        validate_canvas_json(&input.canvas_json)?;
        let now = current_ts();
        let name_for_error = name.clone();

        let txn = self.db.begin().await?;
        let Some(existing) = ViewEntity::find_by_id(id.to_string()).one(&txn).await? else {
            txn.rollback().await?;
            return Err(ViewManagerError::NotFound);
        };

        if input.is_entry_point == Some(false) && existing.is_entry_point {
            txn.rollback().await?;
            return Err(ViewManagerError::Validation(
                "Cannot unset entry-point directly. Set another view as entry-point first."
                    .to_string(),
            ));
        }

        if input.is_entry_point == Some(true) && !existing.is_entry_point {
            let current_entry_points = ViewEntity::find()
                .filter(model::Column::IsEntryPoint.eq(true))
                .filter(model::Column::Id.ne(id.to_string()))
                .all(&txn)
                .await?;
            for entry in current_entry_points {
                let mut active: model::ActiveModel = entry.into();
                active.is_entry_point = Set(false);
                active.updated_at = Set(now);
                active.update(&txn).await?;
            }
        }

        let mut active: model::ActiveModel = existing.into();
        active.name = Set(name);
        active.description = Set(description);
        active.canvas_json = Set(input.canvas_json);
        if let Some(is_entry_point) = input.is_entry_point {
            active.is_entry_point = Set(is_entry_point);
        }
        active.updated_at = Set(now);
        let updated = active
            .update(&txn)
            .await
            .map_err(|error| map_view_name_unique_error(error, &name_for_error))?;

        txn.commit().await?;
        Ok(updated.into())
    }

    pub async fn delete_view(&self, id: &str) -> Result<(), ViewManagerError> {
        let txn = self.db.begin().await?;
        let Some(target) = ViewEntity::find_by_id(id.to_string()).one(&txn).await? else {
            txn.rollback().await?;
            return Err(ViewManagerError::NotFound);
        };

        let count = ViewEntity::find().count(&txn).await?;
        if count <= 1 {
            txn.rollback().await?;
            return Err(ViewManagerError::Validation(
                "Cannot delete the last remaining view".to_string(),
            ));
        }

        ViewEntity::delete_by_id(id.to_string()).exec(&txn).await?;

        if target.is_entry_point {
            let Some(new_entry) = ViewEntity::find()
                .order_by_asc(model::Column::CreatedAt)
                .one(&txn)
                .await?
            else {
                txn.rollback().await?;
                return Err(ViewManagerError::Validation(
                    "At least one view must exist".to_string(),
                ));
            };
            let mut active: model::ActiveModel = new_entry.into();
            active.is_entry_point = Set(true);
            active.updated_at = Set(current_ts());
            active.update(&txn).await?;
        }

        txn.commit().await?;
        Ok(())
    }

    pub async fn set_entry_point(&self, id: &str) -> Result<View, ViewManagerError> {
        let now = current_ts();
        let txn = self.db.begin().await?;

        let all_views = ViewEntity::find().all(&txn).await?;
        if all_views.is_empty() {
            txn.rollback().await?;
            return Err(ViewManagerError::Validation(
                "At least one view must exist".to_string(),
            ));
        }

        let mut target: Option<model::Model> = None;

        for view in all_views {
            let should_be_entry = view.id == id;
            if should_be_entry {
                target = Some(view.clone());
            }
            if view.is_entry_point != should_be_entry {
                let mut active: model::ActiveModel = view.into();
                active.is_entry_point = Set(should_be_entry);
                active.updated_at = Set(now);
                let updated = active.update(&txn).await?;
                if should_be_entry {
                    target = Some(updated);
                }
            }
        }

        let Some(target) = target else {
            txn.rollback().await?;
            return Err(ViewManagerError::NotFound);
        };

        txn.commit().await?;
        Ok(target.into())
    }
}

fn map_view_error(error: ViewManagerError) -> (StatusCode, Json<ApiResponseEmpty>) {
    match error {
        ViewManagerError::NotFound => (
            StatusCode::NOT_FOUND,
            Json(ApiResponseEmpty::error("View not found".to_string())),
        ),
        ViewManagerError::DuplicateName(name) => (
            StatusCode::CONFLICT,
            Json(ApiResponseEmpty::error(format!(
                "View name \"{name}\" already exists"
            ))),
        ),
        ViewManagerError::Validation(message) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponseEmpty::error(message)),
        ),
        ViewManagerError::Database(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error(format!("Database error: {error}"))),
        ),
    }
}

#[utoipa::path(
    get,
    path = "/api/views",
    params(
        ("page" = Option<u64>, Query, description = "1-based page number", example = 1),
        ("page_size" = Option<u64>, Query, description = "Items per page (1-100)", example = 10),
        ("sort_by" = Option<ViewSortBy>, Query, description = "Sort field"),
        ("sort_direction" = Option<SortDirection>, Query, description = "Sort direction"),
        ("search" = Option<String>, Query, description = "Case-insensitive substring match over name and description")
    ),
    responses(
        (status = 200, description = "List views page", body = ApiResponseViewPage)
    ),
    tag = "Views"
)]
pub async fn get_all_views(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ViewListQuery>,
) -> impl IntoResponse {
    match state.views.list_views_page(query).await {
        Ok(page) => (StatusCode::OK, Json(ApiResponse::success(page))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error(format!("Database error: {error}"))),
        )
            .into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/views/{id}",
    responses(
        (status = 200, description = "View found", body = ApiResponseView),
        (status = 404, description = "View not found", body = ApiResponseEmpty)
    ),
    params(
        ("id" = String, Path, description = "View id (UUID)")
    ),
    tag = "Views"
)]
pub async fn get_view(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.views.get_view(&id).await {
        Ok(Some(view)) => (StatusCode::OK, Json(ApiResponse::success(view))).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponseEmpty::error("View not found".to_string())),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error(format!("Database error: {error}"))),
        )
            .into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/api/views",
    request_body = ViewInput,
    responses(
        (status = 200, description = "View created", body = ApiResponseView),
        (status = 409, description = "Duplicate view name", body = ApiResponseEmpty),
        (status = 400, description = "Validation error", body = ApiResponseEmpty)
    ),
    tag = "Views"
)]
pub async fn create_view(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ViewInput>,
) -> impl IntoResponse {
    match state.views.create_view(input).await {
        Ok(view) => (StatusCode::OK, Json(ApiResponse::success(view))).into_response(),
        Err(error) => map_view_error(error).into_response(),
    }
}

#[utoipa::path(
    put,
    path = "/api/views/{id}",
    request_body = ViewInput,
    responses(
        (status = 200, description = "View updated", body = ApiResponseView),
        (status = 404, description = "View not found", body = ApiResponseEmpty),
        (status = 409, description = "Duplicate view name", body = ApiResponseEmpty),
        (status = 400, description = "Validation error", body = ApiResponseEmpty)
    ),
    params(
        ("id" = String, Path, description = "View id (UUID)")
    ),
    tag = "Views"
)]
pub async fn update_view(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(input): Json<ViewInput>,
) -> impl IntoResponse {
    match state.views.update_view(&id, input).await {
        Ok(view) => (StatusCode::OK, Json(ApiResponse::success(view))).into_response(),
        Err(error) => map_view_error(error).into_response(),
    }
}

#[utoipa::path(
    delete,
    path = "/api/views/{id}",
    responses(
        (status = 200, description = "View deleted", body = ApiResponseEmpty),
        (status = 404, description = "View not found", body = ApiResponseEmpty),
        (status = 400, description = "Validation error", body = ApiResponseEmpty)
    ),
    params(
        ("id" = String, Path, description = "View id (UUID)")
    ),
    tag = "Views"
)]
pub async fn delete_view(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.views.delete_view(&id).await {
        Ok(()) => (StatusCode::OK, Json(ApiResponseEmpty::success())).into_response(),
        Err(error) => map_view_error(error).into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/views/entry-point",
    responses(
        (status = 200, description = "Entry-point view", body = ApiResponseView),
        (status = 404, description = "No entry-point view", body = ApiResponseEmpty)
    ),
    tag = "Views"
)]
pub async fn get_entry_point_view(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.views.get_entry_point().await {
        Ok(Some(view)) => (StatusCode::OK, Json(ApiResponse::success(view))).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponseEmpty::error(
                "Entry-point view not found".to_string(),
            )),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error(format!("Database error: {error}"))),
        )
            .into_response(),
    }
}

#[utoipa::path(
    put,
    path = "/api/views/{id}/entry-point",
    responses(
        (status = 200, description = "Entry-point updated", body = ApiResponseView),
        (status = 404, description = "View not found", body = ApiResponseEmpty)
    ),
    params(
        ("id" = String, Path, description = "View id (UUID)")
    ),
    tag = "Views"
)]
pub async fn set_entry_point_view(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.views.set_entry_point(&id).await {
        Ok(view) => (StatusCode::OK, Json(ApiResponse::success(view))).into_response(),
        Err(error) => map_view_error(error).into_response(),
    }
}
