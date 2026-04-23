use chrono::{Duration, Utc};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode, errors::ErrorKind,
};
use serde::{Deserialize, Serialize};

use super::token::TokenClaims;
use crate::http::resources::user::model::Role;

#[derive(Serialize, Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub username: String,
}

impl Session {
    /// Build signed access/refresh JWT session tokens for a user.
    ///
    /// Returns an error instead of panicking if token encoding fails.
    pub fn new(
        username: &str,
        access_secret: &[u8],
        refresh_secret: &[u8],
        access_ttl_secs: u64,
        refresh_ttl_secs: u64,
        role: Role,
    ) -> Result<Self, String> {
        let now = Utc::now();
        let access_exp = (now + Duration::seconds(access_ttl_secs as i64)).timestamp();
        let refresh_exp = (now + Duration::seconds(refresh_ttl_secs as i64)).timestamp();

        let access_claims = TokenClaims {
            user: username.to_string(),
            token_type: "access".to_string(),
            role: role.clone(),
            exp: access_exp,
            issued_at: now.timestamp_millis(),
        };

        let refresh_claims = TokenClaims {
            user: username.to_string(),
            token_type: "refresh".to_string(),
            role,
            exp: refresh_exp,
            issued_at: now.timestamp_millis(),
        };

        let header = Header::new(Algorithm::HS256);
        let access_token = encode(
            &header,
            &access_claims,
            &EncodingKey::from_secret(access_secret),
        )
        .map_err(|e| format!("Failed to encode access token: {e}"))?;
        let refresh_token = encode(
            &header,
            &refresh_claims,
            &EncodingKey::from_secret(refresh_secret),
        )
        .map_err(|e| format!("Failed to encode refresh token: {e}"))?;

        Ok(Session {
            access_token,
            refresh_token,
            username: username.to_string(),
        })
    }
}

pub fn decode_token_claims(
    token: &str,
    secret: &[u8],
) -> Result<TokenClaims, jsonwebtoken::errors::Error> {
    decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::new(Algorithm::HS256),
    )
    .map(|data| data.claims)
}

pub fn is_expired_error(error: &jsonwebtoken::errors::Error) -> bool {
    matches!(error.kind(), ErrorKind::ExpiredSignature)
}
