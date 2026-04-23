pub mod middleware;
pub mod namespace;
pub mod service;
pub mod session;

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use service::{auth_status, login_session_id_post, logout, setup_post};

use crate::http::AppState;

pub fn mount(router: Router<Arc<AppState>>) -> Router<Arc<AppState>> {
    router
        .route("/api/auth/setup", post(setup_post))
        .route("/api/auth/login", post(login_session_id_post))
        .route("/api/auth/status", get(auth_status))
        .route("/api/auth/logout", get(logout))
}
