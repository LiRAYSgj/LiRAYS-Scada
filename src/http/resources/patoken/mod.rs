pub mod model;
pub mod service;

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement};

pub async fn ensure_schema(db: &DatabaseConnection) -> Result<(), DbErr> {
    let patokens_sql = "CREATE TABLE IF NOT EXISTS patokens (\
        id TEXT PRIMARY KEY NOT NULL,\
        name TEXT NOT NULL UNIQUE,\
        user TEXT NOT NULL,\
        role TEXT NOT NULL,\
        token_hash TEXT NOT NULL,\
        expires_at BIGINT NOT NULL\
    );";
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        patokens_sql.to_string(),
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_patokens_name ON patokens(name);".to_string(),
    ))
    .await?;
    Ok(())
}
