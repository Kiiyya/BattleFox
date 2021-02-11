#![feature(arc_new_cyclic)]
use std::{sync::Arc, time::Duration};

use bf4::{Bf4Client, Event};
use rcon::{RconClient, RconConnectionInfo, RconResult};
use tokio::time::Instant;

#[macro_use]
mod macros;

pub mod mapvote;
pub mod rcon;
pub mod bf4;

// async fn handler(bf4: Arc<Bf4Client>, ev: Event) -> rcon::RconResult<()> {
//     match ev {
//         Event::Chat { vis, chatter, msg } => {

//         }
//         Event::Kill { killer, weapon, victim } => {

//         }
//         Event::Spawn { player } => {}
//     }
//     todo!()
// }


async fn busy(i: usize) {
    if i % 1_000_000 == 0 {
        println!("This is i = {}", i);
    }
}

/// This function is only here becuase I like messing with stuff.
/// A general sketchpad. Eventually this crate will be more of a library,
/// And eventually I'll split it up into rcon+bf4client crate, and then
/// Andother crate with everything like mapvote, etc.
#[tokio::main]
async fn main() -> rcon::RconResult<()> {
    // let conn : RconConnectionInfo = ("127.0.0.1", 47200, "smurf").into();
    // let mut bf4 = Arc::new(Bf4Client::new(conn, |_, _| {}).await.unwrap());

    // println!("Enabled events..");

    // // let start = Instant::now();
    // // println!("Time: {}", start.elapsed().as_millis());

    // // bf4
    // //   .addMapVote()
    // //   .addBalancer()
    // //   .add()

    // tokio::time::sleep(Duration::from_secs(60)).await;

    // (Arc::get_mut(&mut bf4))
    //     .unwrap()
    //     .shutdown()
    //     .await?;
    Ok(())
}
