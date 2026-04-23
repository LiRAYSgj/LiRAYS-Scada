use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use rand::{RngCore, rngs::OsRng};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;

use super::model::{self, Entity as PAToken};
use crate::http::resources::user::model::Role;

/// Creates, verifies, and revokes Personal Access Tokens.
pub struct PATokenManager {
    db: DatabaseConnection,
    argon: Argon2<'static>,
}

impl PATokenManager {
    /// Create a manager bound to the shared application DB connection.
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            argon: Argon2::default(),
        }
    }

    /// Create a new PAT and return its cleartext value once.
    ///
    /// The token secret is stored only as Argon2 hash.
    pub async fn create_token(
        &self,
        name: String,
        user: String,
        role: Role,
        expires_at: i64,
    ) -> Result<String, sea_orm::DbErr> {
        let id = Uuid::new_v4().to_string();
        let mut secret_bytes = vec![0u8; 32];
        OsRng.fill_bytes(&mut secret_bytes);
        let secret = URL_SAFE_NO_PAD.encode(secret_bytes);
        let pat = format!("pat_{id}.{secret}");

        let salt = SaltString::generate(&mut OsRng);
        let token_hash = self
            .argon
            .hash_password(secret.as_bytes(), &salt)
            .map_err(|_| sea_orm::DbErr::Custom("Failed to hash token secret".into()))?
            .to_string();

        let active = model::ActiveModel {
            id: Set(id),
            name: Set(name),
            user: Set(user),
            role: Set(role),
            token_hash: Set(token_hash),
            expires_at: Set(expires_at),
        };

        active.insert(&self.db).await?;
        Ok(pat)
    }

    /// Revoke a PAT by id. Returns true when a row was deleted.
    pub async fn revoke_token(&self, id: &str) -> Result<bool, sea_orm::DbErr> {
        let result = PAToken::delete_by_id(id.to_string()).exec(&self.db).await?;
        Ok(result.rows_affected > 0)
    }

    /// Verify a PAT string and return the associated role when valid.
    pub async fn verify(&self, token: &str) -> Result<Role, sea_orm::DbErr> {
        let token = token
            .strip_prefix("pat_")
            .ok_or_else(|| sea_orm::DbErr::Custom("Invalid token format".into()))?;
        let (id, secret) = token
            .split_once('.')
            .ok_or_else(|| sea_orm::DbErr::Custom("Invalid token format".into()))?;
        if id.is_empty() || secret.is_empty() {
            return Err(sea_orm::DbErr::Custom("Invalid token format".into()));
        }

        let token = PAToken::find_by_id(id.to_string())
            .one(&self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::Custom("Token not found".into()))?;
        if token.expires_at <= Utc::now().timestamp() {
            return Err(sea_orm::DbErr::Custom("Token expired".into()));
        }

        let parsed_hash = PasswordHash::new(&token.token_hash)
            .map_err(|_| sea_orm::DbErr::Custom("Invalid token hash".into()))?;

        let secret_matches = self
            .argon
            .verify_password(secret.as_bytes(), &parsed_hash)
            .is_ok();
        if !secret_matches {
            return Err(sea_orm::DbErr::Custom("Invalid token secret".into()));
        }

        Ok(token.role)
    }
}
