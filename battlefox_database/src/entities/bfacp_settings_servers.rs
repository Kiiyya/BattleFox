//! SeaORM Entity. Generated by sea-orm-codegen 0.8.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "bfacp_settings_servers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub server_id: u16,
    #[sea_orm(column_type = "Text", nullable)]
    pub rcon_password: Option<String>,
    pub filter: Option<String>,
    pub monitor_key: Option<u32>,
    pub battlelog_guid: Option<String>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tbl_server::Entity",
        from = "Column::ServerId",
        to = "super::tbl_server::Column::ServerId",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    TblServer,
}

impl Related<super::tbl_server::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TblServer.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
