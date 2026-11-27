//! Relational database selection and connection (SQLite by default, Postgres, MySQL/MariaDB).

use std::path::Path;

use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, DbErr,
    sea_query::{ColumnType, TableCreateStatement},
};
use url::Url;

use crate::settings::settings::{SettingSpec, Settings};

pub const SQLITE_FILE: &str = "static.db";

#[derive(Clone, Debug, Default)]
pub struct DatabaseConfig {
    /// `None` selects the embedded SQLite file in the data directory.
    pub url: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
    pub max_connections: Option<u32>,
}

impl DatabaseConfig {
    pub fn from_settings(settings: &Settings) -> Result<Self, String> {
        let text = |key: &str, env_var: &str| -> Result<Option<String>, String> {
            let value: Option<String> = settings
                .value(&SettingSpec {
                    section: "database",
                    key,
                    env_var,
                    default: None,
                })
                .map_err(|e| format!("Failed to read database.{key}: {e}"))?;
            Ok(value.filter(|v| !v.trim().is_empty()))
        };
        let max_connections: Option<u32> = settings
            .value(&SettingSpec {
                section: "database",
                key: "max_connections",
                env_var: "DATABASE_MAX_CONNECTIONS",
                default: None,
            })
            .map_err(|e| format!("Failed to read database.max_connections: {e}"))?;
        if max_connections == Some(0) {
            return Err("database.max_connections must be greater than 0".into());
        }
        Ok(Self {
            url: text("url", "DATABASE_URL")?,
            user: text("user", "DATABASE_USER")?,
            password: text("password", "DATABASE_PASSWORD")?,
            max_connections,
        })
    }

    /// Builds the connection URL, inserting `user` and `password` percent-encoded.
    pub fn connection_url(&self, data_dir: &Path) -> Result<String, String> {
        let has_credentials = self.user.is_some() || self.password.is_some();
        let Some(raw) = &self.url else {
            if has_credentials {
                return Err("database.user and database.password require database.url".into());
            }
            return Ok(format!(
                "sqlite://{}?mode=rwc",
                data_dir.join(SQLITE_FILE).display()
            ));
        };
        let mut url = Url::parse(raw).map_err(|e| format!("Invalid database.url: {e}"))?;
        match url.scheme() {
            "sqlite" if has_credentials => {
                Err("database.user and database.password are not supported for SQLite".into())
            }
            "sqlite" => Ok(raw.clone()),
            "postgres" | "postgresql" | "mysql" => {
                if let Some(user) = &self.user {
                    url.set_username(user)
                        .map_err(|_| "database.url cannot carry a user".to_string())?;
                }
                if let Some(password) = &self.password {
                    url.set_password(Some(password))
                        .map_err(|_| "database.url cannot carry a password".to_string())?;
                }
                Ok(url.into())
            }
            scheme => Err(format!(
                "Unsupported database.url scheme '{scheme}'; use sqlite, postgres or mysql (also for MariaDB)"
            )),
        }
    }

    pub async fn connect(&self, data_dir: &Path) -> Result<DatabaseConnection, String> {
        let mut options = ConnectOptions::new(self.connection_url(data_dir)?);
        if let Some(max) = self.max_connections {
            options.max_connections(max);
        }
        Database::connect(options)
            .await
            .map_err(|e| format!("Failed to connect database: {e}"))
    }
}

pub async fn create_table_if_missing(
    db: &DatabaseConnection,
    mut table: TableCreateStatement,
) -> Result<(), DbErr> {
    db.execute(table.if_not_exists()).await?;
    Ok(())
}

/// Unbounded text: MySQL `TEXT` stops at 64 KiB, so JSON documents use `LONGTEXT` there.
pub fn large_text(backend: DatabaseBackend) -> ColumnType {
    match backend {
        DatabaseBackend::MySql => ColumnType::custom("longtext"),
        _ => ColumnType::Text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(url: Option<&str>, user: Option<&str>, password: Option<&str>) -> DatabaseConfig {
        DatabaseConfig {
            url: url.map(Into::into),
            user: user.map(Into::into),
            password: password.map(Into::into),
            max_connections: None,
        }
    }

    #[test]
    fn defaults_to_sqlite_in_data_dir() {
        let url = config(None, None, None)
            .connection_url(Path::new("/var/lib/lirays"))
            .unwrap();
        assert_eq!(url, "sqlite:///var/lib/lirays/static.db?mode=rwc");
        assert!(
            config(None, Some("scada"), None)
                .connection_url(Path::new("/tmp"))
                .is_err()
        );
    }

    #[test]
    fn injects_encoded_credentials_and_keeps_options() {
        let url = config(
            Some("postgres://db.local:5432/lirays?sslmode=require"),
            Some("scada"),
            Some("p@ss/w#rd"),
        )
        .connection_url(Path::new("/tmp"))
        .unwrap();
        assert_eq!(
            url,
            "postgres://scada:p%40ss%2Fw%23rd@db.local:5432/lirays?sslmode=require"
        );
    }

    #[test]
    fn rejects_unsupported_schemes_and_sqlite_credentials() {
        let dir = Path::new("/tmp");
        assert!(
            config(Some("mssql://db/lirays"), None, None)
                .connection_url(dir)
                .is_err()
        );
        assert!(
            config(Some("sqlite:///tmp/x.db"), None, Some("secret"))
                .connection_url(dir)
                .is_err()
        );
    }
}
