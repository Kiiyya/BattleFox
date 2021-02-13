use std::{env::var, time::Duration};

use battlefield_rcon::{bf4::{Bf4Client, Event}, rcon::{self, RconClient}};
use tokio::time::sleep;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> rcon::RconResult<()> {
    let ip = var("BFOX_RCON_IP").unwrap_or("127.0.0.1".into());
    let port = var("BFOX_RCON_PORT").unwrap_or("47200".into()).parse::<u16>().unwrap();
    let password = var("BFOX_RCON_PASSWORD").unwrap_or("smurf".into());
    let rcon = RconClient::connect((ip.as_str(), port, password.as_str())).await?;
    let bf4 = Bf4Client::new(rcon).await.unwrap();

    bf4.kill("player").await.unwrap_err();

    let mut event_stream = bf4.event_stream();
    while let Some(ev) = event_stream.next().await {
        match ev {
            Ok(Event::Kill{killer, victim, headshot: _, weapon}) => {
                println!("{} killed {} with a {}!", killer, victim, weapon);
            },
            Ok(Event::Spawn{player, team: _}) => {
                println!("{} spawned", player);
            },
            Ok(_) => {}, // ignore other events.
            Err(err) => {
                println!("Got error: {:?}", err);
            },

        }
    }

    sleep(Duration::from_secs(60)).await;

    Ok(())
}