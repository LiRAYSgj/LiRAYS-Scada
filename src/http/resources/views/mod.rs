pub mod model;
pub mod service;

use std::sync::Arc;

use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{get, post, put},
};
use sea_orm::{DatabaseConnection, DbErr};
use service::{
    create_view, delete_view, get_all_views, get_entry_point_view, get_view, set_entry_point_view,
    update_view,
};

use crate::http::AppState;
use crate::http::resources::auth::middleware::{admin_only, authenticated_only};

pub fn mount(router: Router<Arc<AppState>>, state: Arc<AppState>) -> Router<Arc<AppState>> {
    router
        .route(
            "/api/views",
            get(get_all_views).route_layer(from_fn_with_state(state.clone(), authenticated_only)),
        )
        .route(
            "/api/views",
            post(create_view).route_layer(from_fn_with_state(state.clone(), admin_only)),
        )
        .route(
            "/api/views/{id}",
            get(get_view).route_layer(from_fn_with_state(state.clone(), authenticated_only)),
        )
        .route(
            "/api/views/{id}",
            put(update_view)
                .delete(delete_view)
                .route_layer(from_fn_with_state(state.clone(), admin_only)),
        )
        .route(
            "/api/views/entry-point",
            get(get_entry_point_view)
                .route_layer(from_fn_with_state(state.clone(), authenticated_only)),
        )
        .route(
            "/api/views/{id}/entry-point",
            put(set_entry_point_view).route_layer(from_fn_with_state(state, admin_only)),
        )
}

pub async fn ensure_schema(db: &DatabaseConnection) -> Result<(), DbErr> {
    let manager = service::ViewManager::new(db.clone());
    manager.ensure_initialized().await
}
