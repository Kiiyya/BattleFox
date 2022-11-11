use sqlx::query_as;

use crate::BfoxContext;

#[derive(Debug)]
pub struct AdkatsBattlelogPlayer {
    pub player_id: u32,
    pub persona_id: u64,
    pub user_id: u64,
    pub gravatar: Option<String>,
    pub persona_banned: bool,
}

impl BfoxContext {
    pub async fn get_battlelog_player_by_persona_id(&self, persona: u64) -> Result<Option<AdkatsBattlelogPlayer>, sqlx::Error> {
        pub struct Row {
            pub player_id: u32,
            pub persona_id: u64,
            pub user_id: u64,
            pub gravatar: Option<String>,
            pub persona_banned: i8,
        }

        let res =
            query_as!(Row, "SELECT * from adkats_battlelog_players WHERE persona_id = ?", persona)
            .fetch_optional(&self.pool)
            .await?;

        let res = res.map(|e: Row| AdkatsBattlelogPlayer {
            player_id: e.player_id,
            persona_id: e.persona_id,
            user_id: e.user_id,
            gravatar: e.gravatar,
            persona_banned: e.persona_banned == 0,
        });

        Ok(res)
    }
}