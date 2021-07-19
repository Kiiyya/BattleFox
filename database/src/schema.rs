table! {
    adkats_battlelog_players (player_id) {
        player_id -> Unsigned<Integer>,
        persona_id -> Unsigned<Bigint>,
        user_id -> Unsigned<Bigint>,
        gravatar -> Nullable<Varchar>,
        persona_banned -> Bool,
    }
}