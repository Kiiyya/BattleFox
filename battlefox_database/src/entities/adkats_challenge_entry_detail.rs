//! SeaORM Entity. Generated by sea-orm-codegen 0.8.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "adkats_challenge_entry_detail")]
pub struct Model {
    #[sea_orm(column_name = "EntryID", primary_key, auto_increment = false)]
    pub entry_id: u32,
    #[sea_orm(column_name = "DetailID", primary_key, auto_increment = false)]
    pub detail_id: u32,
    #[sea_orm(column_name = "VictimID")]
    pub victim_id: u32,
    #[sea_orm(column_name = "Weapon")]
    pub weapon: Option<String>,
    #[sea_orm(column_name = "RoundID")]
    pub round_id: u32,
    #[sea_orm(column_name = "DetailTime")]
    pub detail_time: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::adkats_challenge_entry::Entity",
        from = "Column::EntryId",
        to = "super::adkats_challenge_entry::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    AdkatsChallengeEntry,
    #[sea_orm(
        belongs_to = "super::tbl_playerdata::Entity",
        from = "Column::VictimId",
        to = "super::tbl_playerdata::Column::PlayerId",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    TblPlayerdata,
}

impl Related<super::adkats_challenge_entry::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AdkatsChallengeEntry.def()
    }
}

impl Related<super::tbl_playerdata::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TblPlayerdata.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
