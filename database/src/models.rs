use chrono::NaiveDate;
use serde::Serialize;

use crate::schema::bfox_muted_players;

#[derive(Queryable, Serialize)]
#[diesel(table_name="adkats_battlelog_players")]
pub struct AdkatsBattlelogPlayer {
    pub player_id: u32,
    pub persona_id: u64,
    pub user_id: u64,
    pub gravatar: Option<String>,
    pub persona_banned: bool
}

#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name="bfox_muted_players")]
pub struct BfoxMutedPlayer {
    pub eaid: String,
    pub type_: i32,
    pub end_date: Option<NaiveDate>,
    pub kicks: Option<i32>,
    pub reason: Option<String>
}
