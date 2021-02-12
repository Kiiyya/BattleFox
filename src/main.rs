#![feature(arc_new_cyclic)]
#![warn(missing_debug_implementations, rust_2018_idioms)]

use std::time::Duration;

use bf4::Bf4Client;
use rcon::RconClient;
use tokio_stream::StreamExt;

#[macro_use]
pub mod macros;
pub mod bf4;
pub mod mapvote;
pub mod rcon;

/// This function is only here becuase I like messing with stuff.
/// A general sketchpad. Eventually this crate will be more of a library,
/// And eventually I'll split it up into rcon+bf4client crate, and then
/// Andother crate with everything like mapvote, etc.
#[tokio::main]
async fn main() -> rcon::RconResult<()> {
    let rcon = RconClient::connect(("127.0.0.1", 47200, "smurf")).await?;
    let bf4 = Bf4Client::new(rcon).await.unwrap();

    bf4.kill("player").await.unwrap_err();

    let mut e = bf4.event_stream();
    while let Some(ev) = e.next().await {
        println!("main: Got event {:?}", ev);
    }

    tokio::time::sleep(Duration::from_secs(60)).await;

    Ok(())
}
