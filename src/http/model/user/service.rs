use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, PaginatorTrait};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use rand::rngs::OsRng;

use super::model::{self, Entity as User};

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct UserCredentials {
    pub username: String,
    pub password: String,
}

pub struct UserManager {
    db: DatabaseConnection,
    argon: Argon2<'static>,
}

impl UserManager {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            argon: Argon2::default(),
        }
    }

    pub async fn admin_exists(&self) -> Result<bool, sea_orm::DbErr> {
        let count = User::find()
            .filter(model::Column::Username.eq("admin"))
            .count(&self.db)
            .await?;
        Ok(count > 0)
    }

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
            ..Default::default()
        };
        active.insert(&self.db).await?;
        Ok(())
    }

    pub async fn verify(&self, creds: &UserCredentials) -> Result<bool, sea_orm::DbErr> {
        if let Some(user) = User::find()
            .filter(model::Column::Username.eq(creds.username.clone()))
            .one(&self.db)
            .await?
        {
            let parsed_hash = PasswordHash::new(&user.password_hash)
                .map_err(|_| sea_orm::DbErr::Custom("Invalid password hash".into()))?;
            let ok = self
                .argon
                .verify_password(creds.password.as_bytes(), &parsed_hash)
                .is_ok();
            Ok(ok)
        } else {
            Ok(false)
        }
    }
}
