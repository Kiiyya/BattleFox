// #![feature(generic_associated_types)]
// #![feature(arbitrary_self_type)]
#![allow(unused_imports)]

#[macro_use]
extern crate async_trait;

#[macro_use]
extern crate frunk;

use std::{any::TypeId, collections::HashMap, env::var, marker::PhantomData, ops::Deref, sync::Arc, time::Duration};
use ascii::{AsciiChar, AsciiString, IntoAsciiString};
// use cmd::SimpleCommands;
use dotenv::dotenv;
use futures::{Stream, future::BoxFuture};
use mapvote::{Mapvote, ParseMapsResult, parse_maps};
// use rounds::{Rounds, RoundsCtx};
use tokio_stream::StreamExt;

use battlefield_rcon::{bf4::{Bf4Client, Event, Player, Visibility, error::{Bf4Error, Bf4Result}}, rcon::{self, RconClient, RconConnectionInfo, RconError}};
// use maplist::Maplist;
// use mapvote::{parse_maps, Mapvote, ParseMapsResult};
// use lifeguard::Lifeguard;

pub mod maplist;
pub mod mapvote;
// pub mod guard;
// pub mod lifeguard;
mod stv;
// pub mod cmd;
// mod experiments;
// pub mod rounds;


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

    println!("Connecting to {}:{} with password ***...", coninfo.ip, coninfo.port);
    let bf4 = Bf4Client::connect(&coninfo).await.unwrap();
    println!("Connected!");

    let events = bf4.event_stream().await?;
    mapvote_loop(bf4, events).await;

    Ok(())
}

async fn mapvote_loop(bf4: Arc<Bf4Client>, mut events: impl Stream<Item = Result<Event, Bf4Error>> + Unpin) {
    let mapvote = Arc::new(Mapvote::new());

    let jh1 = {
        let mapvote = mapvote.clone();
        let bf4 = bf4.clone();
        tokio::spawn(async move {
            mapvote.spam_status(bf4).await;
            println!("mapvote spammer sutatus done");
        })
    };

    let jh2 = {
        let mapvote = mapvote.clone();
        let bf4 = bf4.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(313)).await;
            mapvote.spam_voting_guide(bf4).await;
            println!("mapvote spammer voting guide done");
        })
    };

    while let Some(event) = events.next().await {
        match event {
            Ok(Event::Chat { vis, player, msg }) => {
                let bf4 = bf4.clone();
                let mapvote = mapvote.clone();
                // fire and forget about it, so we don't block other events. Yay concurrency!
                tokio::spawn(async move {
                    mapvote.handle_chat_msg(bf4, vis, player, msg).await;
                });
            },
            Ok(Event::RoundOver { winning_team: _ }) => {
                let bf4 = bf4.clone();
                let mapvote = mapvote.clone();
                // fire and forget about it, so we don't block other events. Yay concurrency!
                tokio::spawn(async move {
                    mapvote.handle_round_over(bf4).await;
                });
            },
            Err(Bf4Error::Rcon(RconError::ConnectionClosed)) => break,
            _ => {}, // ignore everything else.
        }
    }

    jh1.await.unwrap();
    jh2.await.unwrap();
}
