//! Lists every kind of plugin instances so clients can manage them generically.

use std::sync::Arc;

use axum::{Json, Router, extract::State, middleware::from_fn_with_state, routing::get};

use super::plugin_instances::PluginCollection;
use crate::http::{ApiResponse, AppState, resources::auth::middleware::admin_only};

#[utoipa::path(get, operation_id = "list_plugin_collections", path = "/api/plugin-collections", tag = "Plugin collections", security(("bearerAuth" = [])),
    responses((status = 200, description = "Instance collections with the forms of their compiled-in plugins", body = ApiResponse<Vec<PluginCollection>>)))]
pub async fn list(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Vec<PluginCollection>>> {
    Json(ApiResponse::success(vec![
        state.inbound.collection(),
        state.outbound.collection(),
    ]))
}

pub fn mount(router: Router<Arc<AppState>>, state: Arc<AppState>) -> Router<Arc<AppState>> {
    router.merge(
        Router::new()
            .route("/api/plugin-collections", get(list))
            .route_layer(from_fn_with_state(state, admin_only)),
    )
}
