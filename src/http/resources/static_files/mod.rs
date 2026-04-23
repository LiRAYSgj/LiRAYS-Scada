pub mod service;

use std::sync::Arc;

use axum::Router;

use crate::http::AppState;

pub fn with_fallback(router: Router<Arc<AppState>>) -> Router<Arc<AppState>> {
    router.fallback(service::serve_static)
}
