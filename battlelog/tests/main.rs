use battlelog::apicalls::{ingame_metadata, server_snapshot};

// Sadly we can't use asserts, since the player may not be in the server actually.
#[tokio::test]
async fn get_snapshot() {
    let data = server_snapshot("4d0151b3-81ff-4268-b4e8-5e60d5bc8765".to_string()).await.unwrap();
    // println!("{:#?}", data);

    let player_by_personaid = data.snapshot.get_player_by_personaid(&806262072);
    if player_by_personaid.is_some() {
        println!("Found player by personaid: {:#?}", player_by_personaid.unwrap());
    }

    let player_by_name = data.snapshot.get_player_by_name("xfileFIN");
    if player_by_name.is_some() {
        println!("Found player by name: {:#?}", player_by_name.unwrap());
    }

    // Uncomment to actually display the output of the println!() statements above:
    // panic!();
}

#[tokio::test]
async fn get_ingame_metadata() {
    let meta = ingame_metadata(806262072).await.unwrap();
    assert_eq!(806262072, meta.persona_id);
}
