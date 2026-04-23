pub mod service;

use std::sync::Arc;

use axum::{Router, middleware::from_fn_with_state, routing::get};
use service::ws_handler;

use crate::http::AppState;
use crate::http::resources::auth::middleware::authenticated_only;

pub fn mount(router: Router<Arc<AppState>>, state: Arc<AppState>) -> Router<Arc<AppState>> {
    router.route(
        "/ws",
        get(ws_handler).route_layer(from_fn_with_state(state.clone(), authenticated_only)),
    )
}
