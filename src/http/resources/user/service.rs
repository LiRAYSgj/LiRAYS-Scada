use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use rand::rngs::OsRng;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};

use super::model::{self, Entity as User, Role};

/// Manages user persistence and password hashing/verification.
pub struct UserManager {
    db: DatabaseConnection,
    argon: Argon2<'static>,
}

impl UserManager {
    /// Create a manager bound to the shared application DB connection.
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            argon: Argon2::default(),
        }
    }

    /// Check if the bootstrap admin account already exists.
    pub async fn admin_exists(&self) -> Result<bool, sea_orm::DbErr> {
        let count = User::find()
            .filter(model::Column::Username.eq("admin"))
            .count(&self.db)
            .await?;
        Ok(count > 0)
    }

    /// Create the initial admin account with Argon2-hashed password.
    pub async fn create_admin(&self, password: String) -> Result<(), sea_orm::DbErr> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = self
            .argon
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| sea_orm::DbErr::Custom("Failed to hash password".into()))?
            .to_string();

        let active = model::ActiveModel {
            username: Set("admin".into()),
            password_hash: Set(hash),
            role: Set(Role::Admin),
            ..Default::default()
        };
        active.insert(&self.db).await?;
        Ok(())
    }

    /// Verify a plaintext password against the stored Argon2 hash.
    pub async fn verify_password(
        &self,
        user: &model::Model,
        password: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|_| sea_orm::DbErr::Custom("Invalid password hash".into()))?;
        Ok(self
            .argon
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Fetch a user by username.
    pub async fn get_user(&self, username: &str) -> Result<Option<model::Model>, sea_orm::DbErr> {
        User::find()
            .filter(model::Column::Username.eq(username.to_string()))
            .one(&self.db)
            .await
    }
}
