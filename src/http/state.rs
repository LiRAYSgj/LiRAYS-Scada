use std::{sync::Arc, time::Duration};

use crate::{
    http::resources::{
        auth::session::SessionManager,
        patoken,
        user::{self, model::Role},
        views::service::ViewManager,
    },
    rtdata::variable::VariableManager,
};

#[derive(Clone)]
pub(crate) struct AuthConfig {
    pub(crate) enabled: bool,
    pub(crate) secret: Arc<Vec<u8>>,
    pub(crate) access_ttl: Duration,
    pub(crate) refresh_ttl: Duration,
    pub(crate) secure_cookies: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub(crate) outbound: Arc<super::resources::outbound::service::OutboundManager>,
    pub(crate) inbound: Arc<super::resources::inbound::service::InboundManager>,
    pub(crate) views: Arc<ViewManager>,
    pub(crate) users: Arc<user::service::UserManager>,
    pub(crate) patoken_manager: Arc<patoken::service::PATokenManager>,
    pub(crate) auth: AuthConfig,
    pub(crate) var_manager: Arc<VariableManager>,
    pub(crate) session_manager: Arc<SessionManager>,
}

#[derive(Clone)]
pub struct AuthContext {
    pub role: Role,
}
