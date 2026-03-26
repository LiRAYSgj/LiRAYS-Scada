use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, EntityTrait, QueryOrder, Set};
use sea_orm_migration::MigratorTrait;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::model::{self, Entity as Resource};

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct StaticResource {
    pub id: i32,
    pub name: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct StaticResourceInput {
    pub name: String,
    pub description: String,
}

impl From<model::Model> for StaticResource {
    fn from(model: model::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            description: model.description,
        }
    }
}

pub struct StaticResourceManager {
    db: DatabaseConnection,
}

impl StaticResourceManager {
    pub async fn new(db_file: &str) -> Result<Self, sea_orm::DbErr> {
        let url = format!("sqlite://{}", db_file);
        let db = Database::connect(url).await?;
        Ok(Self { db })
    }

    pub async fn migrate(&self) -> Result<(), sea_orm::DbErr> {
        crate::migration::Migrator::up(&self.db, None).await
    }

    pub async fn create_resource(
        &self,
        input: StaticResourceInput,
    ) -> Result<StaticResource, sea_orm::DbErr> {
        let active = model::ActiveModel {
            name: Set(input.name),
            description: Set(input.description),
            ..Default::default()
        };
        let model = active.insert(&self.db).await?;
        Ok(model.into())
    }

    pub async fn get_resource(
        &self,
        id: i32,
    ) -> Result<Option<StaticResource>, sea_orm::DbErr> {
        let res = Resource::find_by_id(id).one(&self.db).await?;
        Ok(res.map(Into::into))
    }

    pub async fn get_all_resources(&self) -> Result<Vec<StaticResource>, sea_orm::DbErr> {
        let models = Resource::find()
            .order_by_asc(model::Column::Id)
            .all(&self.db)
            .await?;
        Ok(models.into_iter().map(Into::into).collect())
    }

    pub async fn update_resource(
        &self,
        id: i32,
        input: StaticResourceInput,
    ) -> Result<Option<StaticResource>, sea_orm::DbErr> {
        if let Some(mut model) = Resource::find_by_id(id).one(&self.db).await? {
            model.name = input.name;
            model.description = input.description;
            let active: model::ActiveModel = model.into();
            let updated = active.update(&self.db).await?;
            Ok(Some(updated.into()))
        } else {
            Ok(None)
        }
    }

    pub async fn delete_resource(&self, id: i32) -> Result<bool, sea_orm::DbErr> {
        let result = Resource::delete_by_id(id).exec(&self.db).await?;
        Ok(result.rows_affected > 0)
    }
}
