//! SeaORM Entity. Generated by sea-orm-codegen 0.8.0

use super::sea_orm_active_enums::AdkatsRead;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "adkats_records_main")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub record_id: u32,
    pub server_id: u16,
    pub command_type: u32,
    pub command_action: u32,
    pub command_numeric: i32,
    pub target_name: String,
    pub target_id: Option<u32>,
    pub source_name: String,
    pub source_id: Option<u32>,
    pub record_message: String,
    pub record_time: DateTime,
    pub adkats_read: AdkatsRead,
    pub adkats_web: i8,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::adkats_commands::Entity",
        from = "Column::CommandAction",
        to = "super::adkats_commands::Column::CommandId",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    AdkatsCommands2,
    #[sea_orm(
        belongs_to = "super::adkats_commands::Entity",
        from = "Column::CommandType",
        to = "super::adkats_commands::Column::CommandId",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    AdkatsCommands1,
    #[sea_orm(
        belongs_to = "super::tbl_server::Entity",
        from = "Column::ServerId",
        to = "super::tbl_server::Column::ServerId",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    TblServer,
    #[sea_orm(
        belongs_to = "super::tbl_playerdata::Entity",
        from = "Column::SourceId",
        to = "super::tbl_playerdata::Column::PlayerId",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    TblPlayerdata2,
    #[sea_orm(
        belongs_to = "super::tbl_playerdata::Entity",
        from = "Column::TargetId",
        to = "super::tbl_playerdata::Column::PlayerId",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    TblPlayerdata1,
    #[sea_orm(has_many = "super::adkats_bans::Entity")]
    AdkatsBans,
}

impl Related<super::tbl_server::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TblServer.def()
    }
}

impl Related<super::adkats_bans::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AdkatsBans.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
