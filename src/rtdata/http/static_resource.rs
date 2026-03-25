use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sqlx::{sqlite::{SqlitePool, SqliteConnectOptions}, Row};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StaticResource {
    pub id: i32,
    pub name: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StaticResourceInput {
    pub name: String,
    pub description: String,
}

impl From<StaticResourceInput> for StaticResource {
    fn from(input: StaticResourceInput) -> Self {
        StaticResource {
            id: 0, // Will be set by database
            name: input.name,
            description: input.description,
        }
    }
}

pub struct StaticResourceManager {
    pool: SqlitePool,
}

impl StaticResourceManager {
    pub async fn new(db_file: &str) -> Result<Self, sqlx::Error> {
        let schema_path = format!("sqlite://{}", db_file);
        let opts = SqliteConnectOptions::from_str(&schema_path)?.create_if_missing(true);
        let pool = SqlitePool::connect_with(opts).await?;

        let manager = Self { pool };
        manager.init_database().await?;
        Ok(manager)
    }

    async fn init_database(&self) -> Result<(), sqlx::Error> {
        let query = r#"
            CREATE TABLE IF NOT EXISTS static_resources (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT NOT NULL
            );
        "#;

        sqlx::query(query).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn create_resource(&self, input: StaticResourceInput) -> Result<StaticResource, sqlx::Error> {
        let query = "INSERT INTO static_resources (name, description) VALUES (?1, ?2) RETURNING id";
        let row = sqlx::query(query)
            .bind(&input.name)
            .bind(&input.description)
            .fetch_one(&self.pool)
            .await?;

        let id: i32 = row.get(0);

        Ok(StaticResource {
            id,
            name: input.name,
            description: input.description,
        })
    }

    pub async fn get_resource(&self, id: i32) -> Result<Option<StaticResource>, sqlx::Error> {
        let query = "SELECT id, name, description FROM static_resources WHERE id = ?1";
        let row = sqlx::query(query)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => {
                Ok(Some(StaticResource {
                    id: row.get(0),
                    name: row.get(1),
                    description: row.get(2),
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn get_all_resources(&self) -> Result<Vec<StaticResource>, sqlx::Error> {
        let query = "SELECT id, name, description FROM static_resources";
        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await?;

        let mut resources = Vec::new();
        for row in rows {
            resources.push(StaticResource {
                id: row.get(0),
                name: row.get(1),
                description: row.get(2),
            });
        }

        Ok(resources)
    }

    pub async fn update_resource(&self, id: i32, input: StaticResourceInput) -> Result<Option<StaticResource>, sqlx::Error> {
        let query = "UPDATE static_resources SET name = ?1, description = ?2 WHERE id = ?3";
        let result = sqlx::query(query)
            .bind(&input.name)
            .bind(&input.description)
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() > 0 {
            // Fetch the updated resource
            self.get_resource(id).await
        } else {
            Ok(None)
        }
    }

    pub async fn delete_resource(&self, id: i32) -> Result<bool, sqlx::Error> {
        let query = "DELETE FROM static_resources WHERE id = ?1";
        let result = sqlx::query(query)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
