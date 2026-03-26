use sea_orm_migration::prelude::*;

pub struct Migrator;

#[derive(DeriveMigrationName)]
pub struct Migration20240325CreateStaticResources;
#[derive(DeriveMigrationName)]
pub struct Migration20260326CreateUsers;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(Migration20240325CreateStaticResources),
            Box::new(Migration20260326CreateUsers),
        ]
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration20240325CreateStaticResources {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(StaticResources::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StaticResources::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(StaticResources::Name).string().not_null())
                    .col(ColumnDef::new(StaticResources::Description).string().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(StaticResources::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum StaticResources {
    Table,
    Id,
    Name,
    Description,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration20260326CreateUsers {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Users::Username)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Users::PasswordHash).string().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Username,
    PasswordHash,
}
