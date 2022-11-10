use sqlx::query_as;

use crate::{BfoxContext, DateTime};

#[derive(Debug)]
pub struct BfoxMutedPlayer {
    pub eaid: String, // TODO: use the EAID type
    pub type_: i32, // TODO: use an enum.
    pub end_date: Option<DateTime>,
    pub kicks: Option<i32>, // TODO: use u32
    pub reason: Option<String>,
}

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

    // pub fn get_battlelog_player_by_persona_id(conn: &MysqlConnection, persona: &u64) -> Result<AdkatsBattlelogPlayer, diesel::result::Error> {
    //     use schema::adkats_battlelog_players::dsl::{persona_id, adkats_battlelog_players};

    //     adkats_battlelog_players
    //         .filter(persona_id.eq(persona))
    //         // .select(player_id)
    //         .first(conn)
    // }

    // pub fn get_muted_players(conn: &MysqlConnection) -> Result<Vec<BfoxMutedPlayer>, diesel::result::Error> {
    //     use schema::bfox_muted_players::dsl::{type_, end_date, bfox_muted_players};

    //     bfox_muted_players
    //         .filter(type_.eq(MuteType::Days as i32).and(end_date.gt(Some(Utc::now().naive_utc().date()))))
    //         .or_filter(type_.ne(MuteType::Days as i32))
    //         .filter(type_.ne(MuteType::Disabled as i32))
    //         .load(conn)
    // }

    // pub fn get_muted_player(conn: &MysqlConnection, id: &str) -> Result<BfoxMutedPlayer, diesel::result::Error> {
    //     use schema::bfox_muted_players::dsl::{eaid, bfox_muted_players};

    //     bfox_muted_players
    //         .filter(eaid.eq(id))
    //         .first(conn)
    // }

    // pub fn replace_into_muted_player(conn: &MysqlConnection, player: &BfoxMutedPlayer) -> Result<(), Box<dyn Error>> {
    //     use schema::bfox_muted_players::dsl::*;

    //     // let primary_key = player.eaid.clone();
    //     // let exists = bfox_muted_players
    //     //     .filter(eaid.eq(primary_key))
    //     //     .count()
    //     //     .first::<i64>(conn)
    //     //     .unwrap();

    //     // println!("Count: {}", exists);

    //     replace_into(bfox_muted_players)
    //         .values(player)
    //         .execute(conn)?;

    //     Ok(())
    // }

    // pub fn delete_muted_player(conn: &MysqlConnection, id: String) -> Result<(), Box<dyn Error>> {
    //     use schema::bfox_muted_players::dsl::*;

    //     delete(bfox_muted_players)
    //         .filter(eaid.eq(id))
    //         .execute(conn)?;

    //     Ok(())
    // }
}