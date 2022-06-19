//! SeaORM Entity. Generated by sea-orm-codegen 0.8.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tbl_playerdata")]
pub struct Model {
    #[sea_orm(column_name = "PlayerID", primary_key)]
    pub player_id: u32,
    #[sea_orm(column_name = "GameID")]
    pub game_id: u8,
    #[sea_orm(column_name = "ClanTag")]
    pub clan_tag: Option<String>,
    #[sea_orm(column_name = "SoldierName")]
    pub soldier_name: Option<String>,
    #[sea_orm(column_name = "GlobalRank")]
    pub global_rank: u16,
    #[sea_orm(column_name = "PBGUID")]
    pub pbguid: Option<String>,
    #[sea_orm(column_name = "EAGUID")]
    pub eaguid: Option<String>,
    #[sea_orm(column_name = "IP_Address")]
    pub ip_address: Option<String>,
    #[sea_orm(column_name = "DiscordID")]
    pub discord_id: Option<String>,
    #[sea_orm(
        column_name = "IPv6_Address",
        column_type = "Custom(\"VARBINARY(16)\".to_owned())",
        nullable
    )]
    pub i_pv6_address: Option<String>,
    #[sea_orm(column_name = "CountryCode")]
    pub country_code: Option<String>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::adkats_bans::Entity")]
    AdkatsBans,
    #[sea_orm(has_many = "super::adkats_battlecries::Entity")]
    AdkatsBattlecries,
    #[sea_orm(has_many = "super::adkats_battlelog_players::Entity")]
    AdkatsBattlelogPlayers,
    #[sea_orm(has_many = "super::adkats_challenge_entry::Entity")]
    AdkatsChallengeEntry,
    #[sea_orm(has_many = "super::adkats_challenge_entry_detail::Entity")]
    AdkatsChallengeEntryDetail,
    #[sea_orm(has_many = "super::adkats_infractions_global::Entity")]
    AdkatsInfractionsGlobal,
    #[sea_orm(has_many = "super::adkats_infractions_server::Entity")]
    AdkatsInfractionsServer,
    #[sea_orm(has_many = "super::adkats_specialplayers::Entity")]
    AdkatsSpecialplayers,
    #[sea_orm(has_many = "super::adkats_statistics::Entity")]
    AdkatsStatistics,
    #[sea_orm(has_many = "super::adkats_usersoldiers::Entity")]
    AdkatsUsersoldiers,
    #[sea_orm(has_many = "super::bfacp_users_soldiers::Entity")]
    BfacpUsersSoldiers,
    #[sea_orm(has_many = "super::tbl_chatlog::Entity")]
    TblChatlog,
    #[sea_orm(has_many = "super::tbl_playerrank::Entity")]
    TblPlayerrank,
    #[sea_orm(has_many = "super::tbl_server_player::Entity")]
    TblServerPlayer,
}

impl Related<super::adkats_bans::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AdkatsBans.def()
    }
}

impl Related<super::adkats_battlecries::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AdkatsBattlecries.def()
    }
}

impl Related<super::adkats_battlelog_players::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AdkatsBattlelogPlayers.def()
    }
}

impl Related<super::adkats_challenge_entry::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AdkatsChallengeEntry.def()
    }
}

impl Related<super::adkats_challenge_entry_detail::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AdkatsChallengeEntryDetail.def()
    }
}

impl Related<super::adkats_infractions_global::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AdkatsInfractionsGlobal.def()
    }
}

impl Related<super::adkats_infractions_server::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AdkatsInfractionsServer.def()
    }
}

impl Related<super::adkats_specialplayers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AdkatsSpecialplayers.def()
    }
}

impl Related<super::adkats_statistics::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AdkatsStatistics.def()
    }
}

impl Related<super::adkats_usersoldiers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AdkatsUsersoldiers.def()
    }
}

impl Related<super::bfacp_users_soldiers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BfacpUsersSoldiers.def()
    }
}

impl Related<super::tbl_chatlog::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TblChatlog.def()
    }
}

impl Related<super::tbl_playerrank::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TblPlayerrank.def()
    }
}

impl Related<super::tbl_server_player::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TblServerPlayer.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
