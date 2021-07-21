use database::{replace_into_muted_player, delete_muted_player, establish_connection, get_muted_players, models::BfoxMutedPlayer};

fn main() {
    println!("Attempting to connect to database");
    match establish_connection() {
        Ok(con) => {
            println!("Connected to database");

            // Get muted players
            let result = get_muted_players(&con);
            match result {
                Ok(muted_players) => {
                    println!("Muted players: {:#?}", muted_players);
                }
                Err(err) => println!("Error fetching muted players: {}", err),
            }

            // Add (or update) a muted player
            let player = BfoxMutedPlayer {
                eaid: "EA_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string(),
                type_: 1,
                end_date: None,
                kicks: Some(5),
            };
            replace_into_muted_player(&con, player).unwrap();

            // Delete a muted player
            delete_muted_player(&con, "EA_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string()).unwrap();
        },
        Err(error) => panic!("Failed to connect to database: {}", error),
    }
}
