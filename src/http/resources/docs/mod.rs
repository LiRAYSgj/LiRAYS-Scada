pub mod service;

use std::sync::Arc;

use axum::{Router, routing::get};
use service::{openapi_spec, swagger_ui};

use crate::http::AppState;

pub fn mount(router: Router<Arc<AppState>>) -> Router<Arc<AppState>> {
    router
        .route("/api-docs/openapi.json", get(openapi_spec))
        .route("/swagger", get(swagger_ui))
}
