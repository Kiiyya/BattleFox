use std::env::var;

use battlefield_rcon::{bf4::{Bf4Client, Event, error::Bf4Error}, rcon::{self, RconClient, RconError}};
use dotenv::dotenv;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> rcon::RconResult<()> {
    dotenv().ok(); // load (additional) environment variables from `.env` file in working directory.

    let ip = var("BFOX_RCON_IP").unwrap_or("127.0.0.1".into());
    let port = var("BFOX_RCON_PORT").unwrap_or("47200".into()).parse::<u16>().unwrap();
    let password = var("BFOX_RCON_PASSWORD").unwrap_or("smurf".into());
    println!("Connecting to {}:{}...", ip, port);
    let rcon = RconClient::connect((ip.as_str(), port, password.as_str())).await?;
    let bf4 = Bf4Client::new(rcon).await.unwrap();
    println!("Connected!");

    bf4.kill("player").await.unwrap_err();

    // let votes = Vec::new();

    let mut event_stream = bf4.event_stream();
    while let Some(ev) = event_stream.next().await {
        match ev {
            Ok(Event::Kill{killer: Some(killer), victim, headshot: _, weapon}) => {
                println!("{} killed {} with a {}!", killer, victim, weapon);
            },
            Ok(Event::Chat{vis, player, msg}) => {
                println!("{} said \"{}\" with vis {:?}", player, msg, vis);
                if msg.as_str().starts_with("!vote") {
                    // process votes
                    // votes.push(/* something */);
                }
            },
            // Ok(Event::OnRoundOver) => {
            //     bf4.change_map(Map::Pearl_Market).await;
            // }
            Ok(Event::PunkBusterMessage(_)) => {}, // ignore this terrible spam ugh.
            Ok(ev) => {
                println!("Got other event: {:?}", ev);
            },
            Err(Bf4Error::Rcon(RconError::ConnectionClosed)) => {
                break;
            }
            Err(err) => {
                println!("Got error: {:?}", err);
            },
        }
    }

    Ok(())
}