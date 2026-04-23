use serde::{Deserialize, Serialize};

use crate::http::resources::user::model::Role;

#[derive(Serialize, Deserialize)]
pub struct TokenClaims {
    pub user: String,
    #[serde(rename = "type")]
    pub token_type: String,
    pub role: Role,
    pub exp: i64,
    pub issued_at: i64,
}
