pub mod model;
pub mod service;

use axum::{Router, middleware::from_fn_with_state, routing::get};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Statement};
use std::sync::Arc;

use crate::http::{AppState, resources::auth::middleware::admin_only};

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

pub async fn ensure_schema(db: &DatabaseConnection) -> Result<(), DbErr> {
    db.execute(Statement::from_string(
        db.get_database_backend(),
        "CREATE TABLE IF NOT EXISTS ingestors (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            plugin_type TEXT NOT NULL,
            enabled BOOLEAN NOT NULL,
            config_json TEXT NOT NULL
        )"
        .to_string(),
    ))
    .await?;
    Ok(())
}
