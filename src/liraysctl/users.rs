use std::path::PathBuf;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

use super::config::{build_setup_url, load_settings};
use super::db::open_db;
use super::prompt::{hash_password, prompt_and_confirm};
use crate::http::resources::user::model::{self as user_model, Role};

pub(super) async fn remove_admin(config_arg: Option<PathBuf>) -> Result<(), String> {
    let (settings, _used_path) = load_settings(config_arg)?;
    let db = open_db(&settings).await?;

    use user_model::{Column, Entity as User};

    let result = User::delete_many()
        .filter(Column::Username.eq("admin"))
        .exec(&db)
        .await
        .map_err(|e| format!("Failed to delete admin user: {e}"))?;

    let url = build_setup_url(&settings);
    if result.rows_affected > 0 {
        println!(
            "Admin account removed. Open {} in your browser to set a new administrator password.",
            url
        );
    } else {
        println!(
            "No existing admin account was found. Open {} in your browser to set an administrator password.",
            url
        );
    }

    Ok(())
}

pub(super) async fn create_admin(config_arg: Option<PathBuf>) -> Result<(), String> {
    create_user(
        config_arg,
        "admin",
        Role::Admin,
        "Admin password: ",
        "Admin user created. Open the UI and sign in as 'admin' with the new password.",
    )
    .await
}

pub(super) async fn create_operator(config_arg: Option<PathBuf>) -> Result<(), String> {
    create_user(
        config_arg,
        "operator",
        Role::Operator,
        "Operator password: ",
        "Operator user created. Sign in as 'operator' with the new password.",
    )
    .await
}

pub(super) async fn remove_operator(config_arg: Option<PathBuf>) -> Result<(), String> {
    let (settings, _used_path) = load_settings(config_arg)?;
    let db = open_db(&settings).await?;

    use user_model::{Column, Entity as User};

    let result = User::delete_many()
        .filter(Column::Username.eq("operator"))
        .exec(&db)
        .await
        .map_err(|e| format!("Failed to delete operator: {e}"))?;

    if result.rows_affected > 0 {
        println!("Operator user removed.");
    } else {
        println!("Operator user was not present.");
    }
    Ok(())
}

pub(super) async fn update_admin_password(config_arg: Option<PathBuf>) -> Result<(), String> {
    update_password(config_arg, "admin", Role::Admin).await
}

pub(super) async fn update_operator_password(config_arg: Option<PathBuf>) -> Result<(), String> {
    update_password(config_arg, "operator", Role::Operator).await
}

async fn create_user(
    config_arg: Option<PathBuf>,
    username: &str,
    role: Role,
    password_prompt: &str,
    success_message: &str,
) -> Result<(), String> {
    let (settings, _used_path) = load_settings(config_arg)?;
    let db = open_db(&settings).await?;

    use user_model::{ActiveModel, Column, Entity as User};

    if User::find()
        .filter(Column::Username.eq(username))
        .one(&db)
        .await
        .map_err(|e| format!("DB error: {e}"))?
        .is_some()
    {
        return Err(format!(
            "{} user already exists. Use update-{}-password instead.",
            capitalize(username),
            username
        ));
    }

    let password = prompt_and_confirm(password_prompt)?;
    let hash = hash_password(&password)?;

    let active = ActiveModel {
        username: Set(username.to_string()),
        password_hash: Set(hash),
        role: Set(role),
        ..Default::default()
    };
    active
        .insert(&db)
        .await
        .map_err(|e| format!("Failed to create {username}: {e}"))?;

    println!("{success_message}");
    Ok(())
}

async fn update_password(
    config_arg: Option<PathBuf>,
    username: &str,
    role: Role,
) -> Result<(), String> {
    let (settings, _used_path) = load_settings(config_arg)?;
    let db = open_db(&settings).await?;

    use user_model::{ActiveModel, Column, Entity as User};

    let user = User::find()
        .filter(Column::Username.eq(username.to_string()))
        .one(&db)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let Some(existing) = user else {
        return Err(format!("User '{username}' does not exist."));
    };

    let password = prompt_and_confirm(&format!("{username} password: "))?;
    let hash = hash_password(&password)?;

    let mut model: ActiveModel = existing.into();
    model.password_hash = Set(hash);
    model.role = Set(role);

    model
        .update(&db)
        .await
        .map_err(|e| format!("Failed to update {username} password: {e}"))?;

    println!("Password updated for '{username}'.");
    Ok(())
}

fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
