pub mod auth;
pub mod docs;
pub mod resource;
pub mod resources;
pub mod static_files;
pub mod user;
pub mod ws;

pub use auth::service::{
    auth_status, login_api, login_get, login_post, logout, refresh_api, setup_get, setup_post,
};
pub use docs::service::{openapi_spec, swagger_ui};
pub use resources::service::{
    create_resource, delete_resource, get_all_resources, get_resource, update_resource,
};
pub use static_files::service::serve_static;
pub use ws::service::ws_handler;
