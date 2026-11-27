pub mod model;
pub mod service;

use sea_orm::{DatabaseConnection, DbErr, Schema};

use crate::db::create_table_if_missing;

pub async fn ensure_schema(db: &DatabaseConnection) -> Result<(), DbErr> {
    let schema = Schema::new(db.get_database_backend());
    create_table_if_missing(db, schema.create_table_from_entity(model::Entity)).await
}
