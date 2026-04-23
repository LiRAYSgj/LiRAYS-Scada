use sea_orm::{Database, DatabaseConnection};

use super::config::resolve_data_dir;
use crate::http::resources::patoken;
use crate::settings::settings::Settings;

pub(super) async fn open_db(settings: &Settings) -> Result<DatabaseConnection, String> {
    let data_dir = resolve_data_dir(settings)?;
    let db_path = data_dir.join("static.db");

    if !db_path.exists() {
        return Err(format!(
            "SQLite database not found at {}. Is LiRAYS SCADA installed?",
            db_path.display()
        ));
    }

    let db_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());
    Database::connect(&db_url)
        .await
        .map_err(|e| format!("Failed to open database at {}: {e}", db_path.display()))
}

pub(super) async fn ensure_patoken_schema(db: &DatabaseConnection) -> Result<(), String> {
    patoken::ensure_schema(db)
        .await
        .map_err(|e| format!("Failed to ensure patokens schema: {e}"))
}
