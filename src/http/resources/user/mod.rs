pub mod model;
pub mod namespace;
pub mod service;

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement};

pub async fn ensure_schema(db: &DatabaseConnection) -> Result<(), DbErr> {
    let users_sql = "CREATE TABLE IF NOT EXISTS users (\
        id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,\
        username TEXT NOT NULL UNIQUE,\
        password_hash TEXT NOT NULL,\
        role TEXT NOT NULL DEFAULT 'operator'\
    );";
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        users_sql.to_string(),
    ))
    .await?;
    Ok(())
}
