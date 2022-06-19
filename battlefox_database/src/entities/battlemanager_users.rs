//! SeaORM Entity. Generated by sea-orm-codegen 0.8.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "battlemanager_users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub email: String,
    #[sea_orm(unique)]
    pub email_normalized: String,
    pub name: Option<String>,
    pub password: String,
    #[sea_orm(column_type = "Custom(\"BIT(1)\".to_owned())", nullable)]
    pub is_logged_in: Option<String>,
    #[sea_orm(column_type = "Custom(\"BIT(1)\".to_owned())", nullable)]
    pub locked: Option<String>,
    pub created_at: Option<DateTimeUtc>,
    pub updated_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::battlemanager_userroles::Entity")]
    BattlemanagerUserroles,
}

impl Related<super::battlemanager_userroles::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BattlemanagerUserroles.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
