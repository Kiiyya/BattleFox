table! {
    adkats_battlelog_players (player_id) {
        player_id -> Integer,
        persona_id -> BigInt,
        user_id -> BigInt,
        //gravatar -> Varchar,
        persona_banned -> TinyInt,
    }
}
