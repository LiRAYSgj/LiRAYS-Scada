pub mod auth;
pub mod docs;
pub mod resources;
pub mod user;
pub mod ws;
pub mod static_files;
pub mod resource;

pub use auth::service::{login_get, login_post, login_api, refresh_api, logout, setup_get, setup_post};
pub use resources::service::{get_all_resources, get_resource, create_resource, update_resource, delete_resource};
pub use ws::service::ws_handler;
pub use docs::service::{openapi_spec, swagger_ui};
pub use static_files::service::serve_static;
