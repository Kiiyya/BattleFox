//! SeaORM Entity. Generated by sea-orm-codegen 0.8.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tbl_server_player")]
pub struct Model {
    #[sea_orm(column_name = "StatsID", primary_key)]
    pub stats_id: u32,
    #[sea_orm(column_name = "ServerID")]
    pub server_id: u16,
    #[sea_orm(column_name = "PlayerID")]
    pub player_id: u32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tbl_playerdata::Entity",
        from = "Column::PlayerId",
        to = "super::tbl_playerdata::Column::PlayerId",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    TblPlayerdata,
    #[sea_orm(
        belongs_to = "super::tbl_server::Entity",
        from = "Column::ServerId",
        to = "super::tbl_server::Column::ServerId",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    TblServer,
    #[sea_orm(has_many = "super::tbl_playerstats::Entity")]
    TblPlayerstats,
    #[sea_orm(has_many = "super::tbl_sessions::Entity")]
    TblSessions,
    #[sea_orm(has_many = "super::tbl_weapons_stats::Entity")]
    TblWeaponsStats,
}

impl Related<super::tbl_playerdata::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TblPlayerdata.def()
    }
}

impl Related<super::tbl_server::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TblServer.def()
    }
}

impl Related<super::tbl_playerstats::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TblPlayerstats.def()
    }
}

impl Related<super::tbl_sessions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TblSessions.def()
    }
}

impl Related<super::tbl_weapons_stats::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TblWeaponsStats.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
