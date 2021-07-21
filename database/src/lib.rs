#[macro_use]
extern crate diesel;
extern crate dotenv;

use chrono::{Utc};
use shared::mute::MuteType;

pub mod models;
pub mod schema;

use diesel::{delete, prelude::*, replace_into};
use dotenv::dotenv;
use models::BfoxMutedPlayer;

use std::{env, error::Error};

use self::models::{AdkatsBattlelogPlayer};

pub fn establish_connection() -> Result<diesel::MysqlConnection, ConnectionError> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    MysqlConnection::establish(&database_url)
        //.unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn get_battlelog_player_by_persona_id(conn: &MysqlConnection, persona: &u64) -> Result<AdkatsBattlelogPlayer, diesel::result::Error> {
    use schema::adkats_battlelog_players::dsl::{persona_id, adkats_battlelog_players};

    adkats_battlelog_players
        .filter(persona_id.eq(persona))
        // .select(player_id)
        .first(conn)
}

pub fn get_muted_players(conn: &MysqlConnection) -> Result<Vec<BfoxMutedPlayer>, diesel::result::Error> {
    use schema::bfox_muted_players::dsl::{type_, end_date, bfox_muted_players};

    bfox_muted_players
        .filter(type_.eq(MuteType::Days as i32).and(end_date.gt(Some(Utc::now().naive_utc().date()))))
        .or_filter(type_.ne(MuteType::Days as i32))
        .filter(type_.ne(MuteType::Disabled as i32))
        .load(conn)
}

pub fn get_muted_player(conn: &MysqlConnection, id: &str) -> Result<BfoxMutedPlayer, diesel::result::Error> {
    use schema::bfox_muted_players::dsl::{eaid, bfox_muted_players};

    bfox_muted_players
        .filter(eaid.eq(id))
        .first(conn)
}

pub fn replace_into_muted_player(conn: &MysqlConnection, player: BfoxMutedPlayer) -> Result<(), Box<dyn Error>> {
    use schema::bfox_muted_players::dsl::*;

    // let primary_key = player.eaid.clone();
    // let exists = bfox_muted_players
    //     .filter(eaid.eq(primary_key))
    //     .count()
    //     .first::<i64>(conn)
    //     .unwrap();

    // println!("Count: {}", exists);

    replace_into(bfox_muted_players)
        .values(&player)
        .execute(conn)?;

    Ok(())
}

pub fn delete_muted_player(conn: &MysqlConnection, id: String) -> Result<(), Box<dyn Error>> {
    use schema::bfox_muted_players::dsl::*;

    delete(bfox_muted_players)
        .filter(eaid.eq(id))
        .execute(conn)?;

    Ok(())
}
