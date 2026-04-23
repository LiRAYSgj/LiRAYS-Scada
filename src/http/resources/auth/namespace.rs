use serde::{Deserialize, Serialize};

use crate::http::resources::user::model::Role;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatusResponse {
    pub auth_enabled: bool,
    pub authenticated: bool,
    pub admin_exists: bool,
    pub user: Option<UserInfo>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub username: String,
    pub role: Role,
}

#[derive(Deserialize)]
pub struct PasswordForm {
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}
