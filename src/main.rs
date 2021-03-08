// #![feature(generic_associated_types)]
// #![feature(arbitrary_self_type)]
#![allow(unused_imports)]

#[macro_use]
extern crate async_trait;

use std::{any::TypeId, collections::HashMap, env::var, marker::PhantomData, ops::Deref, sync::Arc};
use ascii::{AsciiChar, IntoAsciiString};
use cmd::SimpleCommands;
use dotenv::dotenv;
use futures::future::BoxFuture;
use rounds::{Rounds, RoundsCtx};
use tokio_stream::StreamExt;

use battlefield_rcon::{bf4::{
        error::{Bf4Error, Bf4Result},
        Bf4Client, Event,
    }, rcon::{self, RconClient, RconConnectionInfo, RconError}};
// use maplist::Maplist;
// use mapvote::{parse_maps, Mapvote, ParseMapsResult};
// use lifeguard::Lifeguard;

// pub mod maplist;
// pub mod mapvote;
// pub mod guard;
// pub mod lifeguard;
mod stv;
pub mod cmd;
pub mod rounds;

////////////////////////////////

pub struct Usage<N: Node> {
    _ph: PhantomData<N>
}
impl <N: Node> Usage<N> {
    pub fn with<F: Fn(&mut N::Ctx)>(&mut self, f: F) {
        todo!()
    }
}

pub trait Context {
    fn uses<'ctx, N: Node>(&'ctx mut self) -> &'ctx mut Usage<N>;
}

pub trait Node {
    type Ctx : Context;

    fn define(ctx: &mut Self::Ctx) -> Self
    where
        Self: Sized;
}

pub struct BattleFox<M: Node> {
    bf4: Arc<Bf4Client>,
    // extensions: Vec<Box<dyn Extension>>,
    main: M,
}

pub struct BattleFoxCtx {

}

impl Context for BattleFoxCtx {
    #[must_use]
    fn uses<'ctx, N: Node>(&'ctx mut self) -> &'ctx mut Usage<N> {
        todo!()
    }
}

impl <T: Node<Ctx = BattleFoxCtx>> BattleFox<T> {
    pub async fn run(bf4: Arc<Bf4Client>) -> Self {
        let mut root = BattleFoxCtx {
            // uses: Vec::new(),
        };
        let main = T::define(&mut root);
        Self {
            bf4,
            // extensions: Vec::new(),
            main,
        }
    }

    // pub fn add_ext<T: Extension + 'static>(&mut self) {
    //     let mut scope = ExtUpImpl {
    //         // cmds: Vec::new(),
    //     };
    //     let ext = T::up(&mut scope);
    //     self.extensions.push(Box::new(ext));
    // }

    // pub async fn run<T: Extension>(&mut self) {
    //     let mut events = self.bf4.event_stream();
    //     while let Some(event) = events.next().await {

    //     }
    // }
}

//////////////

struct Main;
impl Node for Main {
    type Ctx = BattleFoxCtx;

    fn define(ctx: &mut BattleFoxCtx) -> Self
    where
        Self: Sized
    {
        ctx.uses::<Rounds>().with(|rounds: &mut RoundsCtx| {
            rounds.uses::<SimpleCommands>();
        });


        // root.uses::<Rounds>(|rounds: &mut RootScope<Rounds>| {
        //     rounds.each::<Mapvote>(|mapvote: &mut RoundScope<MapVote>| {
        //         mapvote.
        //     });
        // });

        Self
    }
}

fn get_rcon_coninfo() -> rcon::RconResult<RconConnectionInfo> {
    let ip = var("BFOX_RCON_IP").unwrap_or("127.0.0.1".into());
    let port = var("BFOX_RCON_PORT")
        .unwrap_or("47200".into())
        .parse::<u16>()
        .unwrap();
    let password = var("BFOX_RCON_PASSWORD").unwrap_or("smurf".into());
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

    BattleFox::<Main>::run(bf4).await;

    Ok(())
}
