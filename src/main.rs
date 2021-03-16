#![allow(unused_imports)]
#![allow(clippy::new_without_default)]

#[macro_use]
extern crate async_trait;

#[macro_use]
extern crate frunk;

use ascii::{AsciiChar, AsciiString, IntoAsciiString};
use std::{
    any::TypeId, collections::HashMap, env::var, marker::PhantomData, ops::Deref, sync::Arc,
    time::Duration,
};
use tokio::sync::mpsc;
// use cmd::SimpleCommands;
use dotenv::dotenv;
use futures::{future::BoxFuture, Stream};
use mapmanager::{MapManager, PopState};
use mapvote::{parse_maps, Mapvote, ParseMapsResult};
// use rounds::{Rounds, RoundsCtx};
use tokio_stream::StreamExt;

use battlefield_rcon::{
    bf4::{
        error::{Bf4Error, Bf4Result},
        Bf4Client, Event, Player, Visibility,
    },
    rcon::{self, RconClient, RconConnectionInfo, RconError},
};
// use maplist::Maplist;
// use mapvote::{parse_maps, Mapvote, ParseMapsResult};
// use lifeguard::Lifeguard;

pub mod mapmanager;
pub mod mapvote;
// pub mod guard;
// pub mod lifeguard;
pub mod cmd;
mod stv;
// mod experiments;
// pub mod frequentive;
// pub mod rounds;
// pub mod admins;
// pub mod vips;
// pub mod minicache;

fn get_rcon_coninfo() -> rcon::RconResult<RconConnectionInfo> {
    let ip = var("BFOX_RCON_IP").unwrap_or_else(|_| "127.0.0.1".into());
    let port = var("BFOX_RCON_PORT")
        .unwrap_or_else(|_| "47200".into())
        .parse::<u16>()
        .unwrap();
    let password = var("BFOX_RCON_PASSWORD").unwrap_or_else(|_| "smurf".into());
    Ok(RconConnectionInfo {
        ip,
        port,
        password: password.into_ascii_string()?,
    })
}

#[allow(clippy::or_fun_call)]
#[tokio::main]
async fn main() -> rcon::RconResult<()> {
    dotenv().ok(); // load (additional) environment variables from `.env` file in working directory.

    let coninfo = get_rcon_coninfo()?;

    println!(
        "Connecting to {}:{} with password ***...",
        coninfo.ip, coninfo.port
    );
    let bf4 = Bf4Client::connect(&coninfo).await.unwrap();
    println!("Connected!");

    let mapman = Arc::new(MapManager::new());
    let mapvote = Arc::new(Mapvote::new(mapman.clone()));

    let mut jhs = Vec::new();

    let bf4clone = bf4.clone();
    jhs.push(tokio::spawn(async move { mapvote.run(bf4clone).await }));

    let bf4clone = bf4.clone();
    jhs.push(tokio::spawn(async move { mapman.run(bf4clone).await }));

    // Wait for all our spawned tasks to finish.
    // This'll happen at shutdown, or never, when you CTRL-C.
    for jh in jhs.drain(..) {
        jh.await.unwrap()?
    }

    Ok(())
}
