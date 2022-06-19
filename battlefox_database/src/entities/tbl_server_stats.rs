//! SeaORM Entity. Generated by sea-orm-codegen 0.8.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tbl_server_stats")]
pub struct Model {
    #[sea_orm(column_name = "ServerID", primary_key, auto_increment = false)]
    pub server_id: u16,
    #[sea_orm(column_name = "CountPlayers")]
    pub count_players: i64,
    #[sea_orm(column_name = "SumScore")]
    pub sum_score: i64,
    #[sea_orm(column_name = "AvgScore")]
    pub avg_score: f32,
    #[sea_orm(column_name = "SumKills")]
    pub sum_kills: i64,
    #[sea_orm(column_name = "AvgKills")]
    pub avg_kills: f32,
    #[sea_orm(column_name = "SumHeadshots")]
    pub sum_headshots: i64,
    #[sea_orm(column_name = "AvgHeadshots")]
    pub avg_headshots: f32,
    #[sea_orm(column_name = "SumDeaths")]
    pub sum_deaths: i64,
    #[sea_orm(column_name = "AvgDeaths")]
    pub avg_deaths: f32,
    #[sea_orm(column_name = "SumSuicide")]
    pub sum_suicide: i64,
    #[sea_orm(column_name = "AvgSuicide")]
    pub avg_suicide: f32,
    #[sea_orm(column_name = "SumTKs")]
    pub sum_t_ks: i64,
    #[sea_orm(column_name = "AvgTKs")]
    pub avg_t_ks: f32,
    #[sea_orm(column_name = "SumPlaytime")]
    pub sum_playtime: i64,
    #[sea_orm(column_name = "AvgPlaytime")]
    pub avg_playtime: f32,
    #[sea_orm(column_name = "SumRounds")]
    pub sum_rounds: i64,
    #[sea_orm(column_name = "AvgRounds")]
    pub avg_rounds: f32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tbl_server::Entity",
        from = "Column::ServerId",
        to = "super::tbl_server::Column::ServerId",
        on_update = "NoAction",
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
