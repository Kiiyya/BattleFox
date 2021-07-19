use battlelog::apicalls::{ingame_metadata, server_snapshot};


#[tokio::main]
async fn main() {
    // Server snapshot test
    match server_snapshot("4d0151b3-81ff-4268-b4e8-5e60d5bc8765".to_string()).await {
        Ok(data) => {
            //println!("{:#?}", data);

            let player_by_personaid = data.snapshot.get_player_by_personaid(&806262072);
            if player_by_personaid.is_some() {
                println!("Found player by personaid: {:#?}", player_by_personaid.unwrap());
            }

            let player_by_name = data.snapshot.get_player_by_name("xfileFIN");
            if player_by_name.is_some() {
                println!("Found player by name: {:#?}", player_by_name.unwrap());
            }
        },
        Err(error) => println!("Error fetching snapshot: {}", error),
    }

    // Ingame metadata test
    match ingame_metadata(806262072).await {
        Ok(data) => {
            println!("{:#?}", data);
        },
        Err(error) => println!("Error fetching ingame metadata: {}", error),
    }
}
