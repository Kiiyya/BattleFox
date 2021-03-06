use std::{env::var, sync::Arc};
use ascii::AsciiChar;
use dotenv::dotenv;
use tokio::time::sleep;
use tokio_stream::StreamExt;

use battlefield_rcon::{bf4::{Bf4Client, Event, error::{Bf4Error, Bf4Result}}, rcon::{self, RconClient, RconError}};
use mapvote::{Mapvote, ParseMapsResult, parse_maps};
use maplist::Maplist;

pub mod mapvote;
pub mod maplist;
// pub mod guard;
mod stv;

// pub struct BattleFox {
//     bf4: Arc<Bf4Client>,
// }

// impl BattleFox {
//     pub async fn new(bf4: Arc<Bf4Client>) -> Self {
//         Self {
//             bf4,
//         }
//     }

//     pub async fn run(&mut self) -> Bf4Result<()> {

//         Ok(())
//     }
// }

#[allow(clippy::or_fun_call)]
#[tokio::main]
async fn main() -> rcon::RconResult<()> {
    dotenv().ok(); // load (additional) environment variables from `.env` file in working directory.

    let ip = var("BFOX_RCON_IP").unwrap_or("127.0.0.1".into());
    let port = var("BFOX_RCON_PORT")
        .unwrap_or("47200".into())
        .parse::<u16>()
        .unwrap();
    let password = var("BFOX_RCON_PASSWORD").unwrap_or("smurf".into());
    println!("Connecting to {}:{}...", ip, port);
    let rcon = RconClient::connect((ip.as_str(), port, password.as_str())).await?;
    let bf4 = Bf4Client::new(rcon).await.unwrap();
    println!("Connected!");

    tokio::spawn(async move {
        let mapvote = Arc::new(Mapvote::new());

        let mut event_stream = bf4.event_stream();
        while let Some(ev) = event_stream.next().await {
            match ev {
                Ok(Event::Chat { vis, player, msg }) => {
                    if msg[0] == AsciiChar::Exclamation || msg[0] == AsciiChar::Slash {
                        match parse_maps(&msg[1..].as_str()) {
                            ParseMapsResult::Ok(maps) => {
                                mapvote.vote(&player, &maps).await;
                                let _ = bf4.say(format!("{} voted for {:?}", player, maps), vis).await;
                            }
                            ParseMapsResult::NotAMapName { orig } => { // actually error
                                let _ = bf4.say(format!("{}: \"{}\" is not a map name, try again!", player, orig), player).await;
                            }
                            ParseMapsResult::Nothing => { // mapvote didn't consume event, so now we handle all other commands.
                                let words = msg[1..].as_str().split(" ").collect::<Vec<_>>();
                                match words[0] {
                                    "!v" | "/v" => {
                                        bf4.say_lines(mapvote.format_status(), vis).await.unwrap();
                                    }
                                    // "/nominate" | "!nominate" | "/nom" | "!nom" => {
                                    //     if msg.len() == 1 {
                                    //         bf4.say_lines(vec![
                                    //             "Usage: \"!nominate metro\" or \"/nom metro@rush\" or \"/nom pearl\"",
                                    //             "Usage: Adds a map to the current vote, so everyone can vote on it!",
                                    //             "Usage: For more details see \"/help nom\""
                                    //         ], Visibility::Player(player.into())).await.unwrap();
                                    //     }
                                    //     // now we know msg.len >= 2.
                                    // }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Ok(Event::RoundOver { winning_team: _ }) => {
    
                }
                Ok(_) => {}
                // Ok(ev) => {
                //     // let end = Box::pin(async move |_, _| Ok(()));
                //     let mapvote = mapvote.clone();
                //     let bf4 = bf4.clone();
                //     tokio::spawn(async move {
                //         if let Err(e) = mapvote.event(bf4, ev).await {
                //             // TODO: handle joinhandle errors properly, not just logging, bleh.
                //             println!("Got a middleware error {:?}", e);
                //         }
                //     });
                // }
                Err(Bf4Error::Rcon(RconError::ConnectionClosed)) => {
                    break;
                }
                Err(err) => {
                    println!("Got error: {:?}", err);
                    if let Bf4Error::Rcon(RconError::Io(_)) = err {
                        break;
                    }
                }
            }
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    Ok(())
}
