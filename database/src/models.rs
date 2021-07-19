use serde::Serialize;

#[derive(Queryable, Serialize)]
#[diesel(table_name="adkats_battlelog_players")]
pub struct AdkatsBattlelogPlayer {
    pub player_id: u32,
    pub persona_id: u64,
    pub user_id: u64,
    pub gravatar: Option<String>,
    pub persona_banned: bool
}
