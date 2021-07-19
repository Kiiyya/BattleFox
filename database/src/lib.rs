#[macro_use]
extern crate diesel;
extern crate dotenv;

pub mod models;
pub mod schema;

use diesel::prelude::*;
use dotenv::dotenv;

use std::env;

use self::models::{AdkatsBattlelogPlayer};

pub fn establish_connection() -> Result<diesel::MysqlConnection, ConnectionError> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    MysqlConnection::establish(&database_url)
        //.unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn get_battlelog_player_by_persona_id(conn: &MysqlConnection, persona: &i64) -> Result<AdkatsBattlelogPlayer, diesel::result::Error> {
    use schema::adkats_battlelog_players::dsl::{persona_id, adkats_battlelog_players};

    adkats_battlelog_players
        .filter(persona_id.eq(persona))
        // .select(player_id)
        .first(conn)
}