//! SeaORM Entity. Generated by sea-orm-codegen 0.8.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "adkats_rolegroups")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub role_id: u32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub group_key: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::adkats_roles::Entity",
        from = "Column::RoleId",
        to = "super::adkats_roles::Column::RoleId",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    AdkatsRoles,
}

impl Related<super::adkats_roles::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AdkatsRoles.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
