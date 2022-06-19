//! SeaORM Entity. Generated by sea-orm-codegen 0.8.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "bfacp_assigned_roles")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u32,
    pub user_id: u32,
    pub role_id: u32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::bfacp_roles::Entity",
        from = "Column::RoleId",
        to = "super::bfacp_roles::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    BfacpRoles,
    #[sea_orm(
        belongs_to = "super::bfacp_users::Entity",
        from = "Column::UserId",
        to = "super::bfacp_users::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    BfacpUsers,
}

impl Related<super::bfacp_roles::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BfacpRoles.def()
    }
}

impl Related<super::bfacp_users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BfacpUsers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
