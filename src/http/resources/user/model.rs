use sea_orm::entity::prelude::*;

#[derive(
    Clone, Debug, PartialEq, Eq, DeriveActiveEnum, EnumIter, serde::Serialize, serde::Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "String(Some(16))")]
pub enum Role {
    #[sea_orm(string_value = "admin")]
    #[serde(rename = "admin")]
    Admin,
    #[sea_orm(string_value = "operator")]
    #[serde(rename = "operator")]
    Operator,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, serde::Serialize, serde::Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub username: String,
    pub password_hash: String,
    pub role: Role,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
