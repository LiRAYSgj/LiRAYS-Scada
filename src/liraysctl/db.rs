use sea_orm::DatabaseConnection;

use super::config::resolve_data_dir;
use crate::db::{DatabaseConfig, SQLITE_FILE};
use crate::http::resources::patoken;
use crate::settings::settings::Settings;

pub(super) async fn open_db(settings: &Settings) -> Result<DatabaseConnection, String> {
    let data_dir = resolve_data_dir(settings)?;
    let database = DatabaseConfig::from_settings(settings)?;

    if database.url.is_none() {
        let db_path = data_dir.join(SQLITE_FILE);
        if !db_path.exists() {
            return Err(format!(
                "SQLite database not found at {}. Is LiRAYS SCADA installed?",
                db_path.display()
            ));
        }
    }

    database.connect(&data_dir).await
}

pub(super) async fn ensure_patoken_schema(db: &DatabaseConnection) -> Result<(), String> {
    patoken::ensure_schema(db)
        .await
        .map_err(|e| format!("Failed to ensure patokens schema: {e}"))
}
