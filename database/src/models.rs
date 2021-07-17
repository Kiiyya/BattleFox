use super::schema::adkats_battlelog_players;
use serde::Serialize;

#[derive(Queryable, Serialize)]
#[diesel(table_name="adkats_battlelog_players")]
pub struct AdkatsBattlelogPlayer {
    pub player_id: i32,
    pub persona_id: i64,
    pub user_id: i64,
    //pub gravatar: String,
    pub persona_banned: i8
}
