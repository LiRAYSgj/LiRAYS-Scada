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
        let expiration = |ttl_secs: u64| {
            i64::try_from(ttl_secs)
                .ok()
                .and_then(Duration::try_seconds)
                .and_then(|ttl| now.checked_add_signed(ttl))
                .map(|exp| exp.timestamp())
                .ok_or_else(|| format!("Session TTL of {ttl_secs} seconds is out of range"))
        };
        let access_exp = expiration(access_ttl_secs)?;
        let refresh_exp = expiration(refresh_ttl_secs)?;

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
        let access_token = EncodingKey::from_secret(access_secret)
            .and_then(|key| encode(&header, &access_claims, &key))
            .map_err(|e| format!("Failed to encode access token: {e}"))?;
        let refresh_token = EncodingKey::from_secret(refresh_secret)
            .and_then(|key| encode(&header, &refresh_claims, &key))
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
    let key = DecodingKey::from_secret(secret)?;
    decode::<TokenClaims>(token, &key, &Validation::new(Algorithm::HS256)).map(|data| data.claims)
}

pub fn is_expired_error(error: &jsonwebtoken::errors::Error) -> bool {
    matches!(error.kind(), ErrorKind::ExpiredSignature)
}
