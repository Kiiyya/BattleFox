//! SeaORM Entity. Generated by sea-orm-codegen 0.8.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tbl_playerrank")]
pub struct Model {
    #[sea_orm(column_name = "PlayerID", primary_key, auto_increment = false)]
    pub player_id: u32,
    #[sea_orm(column_name = "ServerGroup", primary_key, auto_increment = false)]
    pub server_group: u16,
    #[sea_orm(column_name = "rankScore")]
    pub rank_score: u32,
    #[sea_orm(column_name = "rankKills")]
    pub rank_kills: u32,
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
}

impl Related<super::tbl_playerdata::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TblPlayerdata.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
