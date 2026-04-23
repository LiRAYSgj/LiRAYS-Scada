use std::path::PathBuf;

use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use super::config::load_settings;
use super::db::{ensure_patoken_schema, open_db};
use super::prompt::{
    print_table, prompt_expiration_seconds, prompt_revoke_target, prompt_token_name,
};
use crate::http::resources::patoken::{model as patoken_model, service::PATokenManager};
use crate::http::resources::user::model::{Column as UserColumn, Entity as UserEntity, Role};

pub(super) async fn generate_admin_token(config_arg: Option<PathBuf>) -> Result<(), String> {
    generate_token(config_arg, "admin", Role::Admin).await
}

pub(super) async fn generate_operator_token(config_arg: Option<PathBuf>) -> Result<(), String> {
    generate_token(config_arg, "operator", Role::Operator).await
}

pub(super) async fn list_tokens(config_arg: Option<PathBuf>) -> Result<(), String> {
    let (settings, _used_path) = load_settings(config_arg)?;
    let db = open_db(&settings).await?;
    ensure_patoken_schema(&db).await?;

    use patoken_model::{Column, Entity as PAToken};

    let tokens = PAToken::find()
        .order_by_asc(Column::User)
        .order_by_asc(Column::Name)
        .order_by_asc(Column::Id)
        .all(&db)
        .await
        .map_err(|e| format!("Failed to list PAT tokens: {e}"))?;

    if tokens.is_empty() {
        println!("No PAT tokens found.");
        return Ok(());
    }

    let now = Utc::now().timestamp();
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(tokens.len());
    for token in tokens {
        let expires_at = chrono::DateTime::from_timestamp(token.expires_at, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| token.expires_at.to_string());
        let status = if token.expires_at <= now {
            "expired"
        } else {
            "active"
        };
        rows.push(vec![
            token.id,
            token.name,
            token.user,
            format!("{:?}", token.role).to_lowercase(),
            expires_at,
            status.to_string(),
        ]);
    }

    print_table(
        &["id", "name", "user", "role", "expires_at", "status"],
        &rows,
    );

    Ok(())
}

pub(super) async fn revoke_token(config_arg: Option<PathBuf>) -> Result<(), String> {
    let (settings, _used_path) = load_settings(config_arg)?;
    let db = open_db(&settings).await?;
    ensure_patoken_schema(&db).await?;

    let target = prompt_revoke_target()?;
    use patoken_model::{Column, Entity as PAToken};

    let id = if let Some(token) = PAToken::find_by_id(target.clone())
        .one(&db)
        .await
        .map_err(|e| format!("Failed to find token by id: {e}"))?
    {
        token.id
    } else if let Some(token) = PAToken::find()
        .filter(Column::Name.eq(target.clone()))
        .one(&db)
        .await
        .map_err(|e| format!("Failed to find token by name: {e}"))?
    {
        token.id
    } else {
        return Err(format!("Token '{target}' was not found by id or name."));
    };

    let manager = PATokenManager::new(db);
    let revoked = manager
        .revoke_token(&id)
        .await
        .map_err(|e| format!("Failed to revoke token: {e}"))?;
    if revoked {
        println!("Token revoked: {id}");
        Ok(())
    } else {
        Err(format!("Token '{target}' was not found by id or name."))
    }
}

async fn generate_token(
    config_arg: Option<PathBuf>,
    username: &str,
    role: Role,
) -> Result<(), String> {
    let (settings, _used_path) = load_settings(config_arg)?;

    let db = open_db(&settings).await?;
    let user = UserEntity::find()
        .filter(UserColumn::Username.eq(username.to_string()))
        .one(&db)
        .await
        .map_err(|e| format!("DB error: {e}"))?;
    if user.is_none() {
        return Err(format!(
            "User '{username}' does not exist. Create it first."
        ));
    }

    ensure_patoken_schema(&db).await?;
    use patoken_model::{Column as PATokenColumn, Entity as PAToken};

    let manager = PATokenManager::new(db.clone());
    let exp = prompt_expiration_seconds()?;
    let name = prompt_token_name()?;
    let already_exists = PAToken::find()
        .filter(PATokenColumn::Name.eq(name.clone()))
        .one(&db)
        .await
        .map_err(|e| format!("Failed to validate token name uniqueness: {e}"))?
        .is_some();
    if already_exists {
        return Err(format!("Token name '{name}' already exists."));
    }
    let token = manager
        .create_token(name.clone(), username.to_string(), role, exp)
        .await
        .map_err(|e| format!("Failed to create PAT token: {e}"))?;

    println!("────────────────────────────────────────────────────────────────────");
    println!("                PAT token created successfully");
    println!("┌──────────────────────────────────────────────────────────────┐");
    println!("│ IMPORTANT: Copy this token now.                              │");
    println!("│ This is the only time it will be shown in full.              │");
    println!("└──────────────────────────────────────────────────────────────┘");
    println!("User: {username}");
    println!("Name: {name}");
    println!();
    println!("Token: {token}");
    if let Some(dt) = chrono::DateTime::from_timestamp(exp, 0) {
        println!("Expires at: {}", dt.to_rfc3339());
    } else {
        println!("Expires at: <invalid timestamp>");
    }
    println!("Use as: Authorization: Bearer <token>");
    println!("────────────────────────────────────────────────────────────────────");
    println!();

    Ok(())
}
