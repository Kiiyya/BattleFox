// #![feature(generic_associated_types)]
#![feature(arbtrary_self_type)]
#![allow(unused_imports)]

#[macro_use]
extern crate async_trait;

use std::{any::TypeId, collections::HashMap, env::var, marker::PhantomData, ops::Deref, sync::Arc};
use ascii::AsciiChar;
use dotenv::dotenv;
use futures::future::BoxFuture;
use tokio_stream::StreamExt;

use battlefield_rcon::{
    bf4::{
        error::{Bf4Error, Bf4Result},
        Bf4Client, Event,
    },
    rcon::{self, RconClient, RconError},
};
use maplist::Maplist;
use mapvote::{parse_maps, Mapvote, ParseMapsResult};
use lifeguard::Lifeguard;

pub mod maplist;
pub mod mapvote;
// pub mod guard;
pub mod lifeguard;
mod stv;
pub mod cmd;
pub mod rounds;

////////////////////////////////

pub trait Context {
    fn has<T>(&mut self, data: T);
}

pub trait Scoped<T, S: Context> : Deref<Target = T>
    // where Self::Target: T,
{
    // type Target = T;
}

struct Usage {
    ty: TypeId,
    f: Box<dyn Fn() -> BoxFuture<'static, ()>>,
}
pub struct RootContext {
    uses: Vec<Usage>,
}
impl Context for RootContext {
    fn has<T>(&mut self, data: T) {
        todo!()
    }
}

pub trait ExtUp {
    fn uses<T: Extension, S: Context, F: Fn(&S) -> Bf4Result<()>>(&mut self, f: F);

    // fn composition(&mut self);

    fn has<T>(&mut self, data: T);

    // fn has_persistent<T: Data>(&mut self);
    // no, no persistent. Instead, you store data in a scope. And maybe you want to store data
    // in your parent scope, which lives longer.
}


pub trait Extension {
    fn define(scope: &mut impl ExtUp) -> Self
    where
        Self: Sized;
}

pub struct BattleFox<T: Extension> {
    bf4: Arc<Bf4Client>,
    // extensions: Vec<Box<dyn Extension>>,
    main: T,
}

impl <T: Extension> BattleFox<T> {
    pub async fn run(bf4: Arc<Bf4Client>) -> Self {
        let root = RootContext {
            uses: Vec::new(),
        };
        let main = T::define(&root);
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
impl Extension for Main {
    fn define(scope: &mut impl ExtUp) -> Self
    where
        Self: Sized
    {
        // scope.uses::<InitScope<Mapvote>>(|&mut mv| {
        //      mv.has_setting(...);
        // });
        
        scope.uses::<Rounds>(|rounds: &mut RootScope<Rounds>| {
            rounds.each::<Mapvote>(|mapvote: &mut RoundScope<MapVote>| {
                mapvote.
            });
        });

        scope.uses::<Mapvote>(|&mut mv| {
            // mv.
        });

        Self
    }
}

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

    println!("Connecting to {}:{} with password ***...", ip, port);
    let rcon = RconClient::connect((ip.as_str(), port, password.as_str())).await?;
    let bf4 = Bf4Client::new(rcon).await.unwrap();
    println!("Connected!");

    BattleFox::<Main>::run(bf4).await;

    Ok(())
}
