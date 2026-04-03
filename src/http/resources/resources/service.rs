use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::http::resources::resource::service::StaticResourceInput;
use crate::http::{ApiResponse, ApiResponseEmpty, AppState};

#[utoipa::path(
    get,
    path = "/api/resources/{id}",
    responses(
        (status = 200, description = "Resource found", body = ApiResponseResource),
        (status = 404, description = "Resource not found", body = ApiResponseEmpty)
    ),
    params(
        ("id" = i32, Path, description = "Resource id")
    )
)]
pub async fn get_resource(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match state.resources.get_resource(id).await {
        Ok(Some(resource)) => {
            (StatusCode::OK, Json(ApiResponse::success(resource))).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponseEmpty::error("Resource not found".to_string())),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error(format!("Database error: {}", e))),
        )
            .into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/resources",
    responses(
        (status = 200, description = "List resources", body = ApiResponseResourceList)
    )
)]
pub async fn get_all_resources(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.resources.get_all_resources().await {
        Ok(resources) => (StatusCode::OK, Json(ApiResponse::success(resources))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error(format!("Database error: {}", e))),
        )
            .into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/api/resources",
    request_body = StaticResourceInput,
    responses(
        (status = 200, description = "Resource created", body = ApiResponseResource),
        (status = 400, description = "Database error", body = ApiResponseEmpty)
    )
)]
pub async fn create_resource(
    State(state): State<Arc<AppState>>,
    Json(input): Json<StaticResourceInput>,
) -> impl IntoResponse {
    match state.resources.create_resource(input).await {
        Ok(resource) => (StatusCode::OK, Json(ApiResponse::success(resource))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error(format!("Database error: {}", e))),
        )
            .into_response(),
    }
}

#[utoipa::path(
    put,
    path = "/api/resources/{id}",
    request_body = StaticResourceInput,
    responses(
        (status = 200, description = "Resource updated", body = ApiResponseResource),
        (status = 404, description = "Not found", body = ApiResponseEmpty),
        (status = 400, description = "Database error", body = ApiResponseEmpty)
    ),
    params(
        ("id" = i32, Path, description = "Resource id")
    )
)]
pub async fn update_resource(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(input): Json<StaticResourceInput>,
) -> impl IntoResponse {
    match state.resources.update_resource(id, input).await {
        Ok(Some(resource)) => (StatusCode::OK, Json(ApiResponse::success(resource))).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponseEmpty::error("Resource not found".to_string())),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error(format!("Database error: {}", e))),
        )
            .into_response(),
    }
}

#[utoipa::path(
    delete,
    path = "/api/resources/{id}",
    responses(
        (status = 200, description = "Resource deleted", body = ApiResponseEmpty),
        (status = 404, description = "Not found", body = ApiResponseEmpty),
        (status = 400, description = "Database error", body = ApiResponseEmpty)
    ),
    params(
        ("id" = i32, Path, description = "Resource id")
    )
)]
pub async fn delete_resource(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match state.resources.delete_resource(id).await {
        Ok(true) => (StatusCode::OK, Json(ApiResponseEmpty::success())).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponseEmpty::error("Resource not found".to_string())),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error(format!("Database error: {}", e))),
        )
            .into_response(),
    }
}
