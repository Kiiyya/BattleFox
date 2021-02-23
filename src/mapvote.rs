#![allow(unused_variables, unused_imports)]

use crate::stv::Profile;

use super::stv::Ballot;
use battlefield_rcon::{bf4::{Bf4Client, Event, GameMode, Map, Player, Visibility, error::Bf4Result}, rcon::RconResult};
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    pin::Pin,
    sync::{Arc, Weak},
    time::Duration,
};
use tokio::{sync::Mutex, time::Interval};

pub type Alt = (Map, GameMode);

#[derive(Debug)]
struct MapvoteInner {
    alternatives: HashSet<Alt>,
    votes: HashMap<Player, Ballot<Alt>>,
}

#[derive(Debug)]
pub struct Mapvote {
    inner: Mutex<MapvoteInner>,
}

impl Mapvote {
    pub async fn init(_bf4: &Arc<Bf4Client>) -> Self {
        // bf4.mk_interval(tokio::time::Duration::from_secs(15), async { |bf4: &Weak<Bf4Client>| async {} });
        Self {
            inner: Mutex::new(MapvoteInner {
                alternatives: HashSet::new(),
                votes: HashMap::new(),
            }),
        }
    }

    pub async fn format_status(&self) -> String {
        let mut ret = String::new();

        

        ret
    }

    /// returns Some(old_ballot) if player had voted before.
    pub async fn vote(&self, player: Player, ballot: Ballot<Alt>) -> Option<Ballot<Alt>> {
        let mut lock = self.inner.lock().await;
        lock.votes.insert(player, ballot)
    }

    pub async fn compute_result(&self) -> Option<Alt> {
        let profile = {
            let lock = self.inner.lock().await;
            Profile {
                alts: lock.alternatives.clone(),
                ballots: lock.votes.values().cloned().collect(),
            }
            // mutex gets dropped here, before we run STV.
        };
        profile.vanilla_stv_1()
    }

    pub async fn event(&self, bf4: Arc<Bf4Client>, ev: Event) -> Bf4Result<()> {
        match ev {
            Event::Chat { vis, player, msg } => {
                let msg : Vec<_> = msg.as_str().split(" ").collect(); // split guarantees at least one elem.
                let firstword = msg[0];
                match firstword {
                    "!v" | "/v" => {
                        
                    }
                    "/nominate" | "!nominate" | "/nom" | "!nom" => {
                        if msg.len() == 1 {
                            bf4.say_lines(vec![
                                "Usage: \"!nominate metro\" or \"/nom metro@rush\" or \"/nom pearl\"",
                                "Usage: Adds a map to the current vote, so everyone can vote on it!",
                                "Usage: For more details see \"/help nom\""
                            ], Visibility::Player(player.into())).await.unwrap();
                        }
                    }
                    _ => {}
                }
            }
            Event::RoundOver { winning_team } => {
                let winner = self.compute_result().await;
                bf4.say(
                    format!("{:?} won the mapvote!", winner.unwrap()),
                    Visibility::All,
                )
                .await
                .unwrap();

                // TODO actually switch to the new map
            }
            _ => {} // ignore other stuff
        }

        Ok(())
    }
}

// use super::Middleware;

// #[async_trait::async_trait]
// impl Middleware for Mapvote {
//     async fn event(
//         &self,
//         bf4: Arc<Bf4Client>,
//         ev: Event,
//         inner: impl FnOnce(Arc<Bf4Client>, Event) -> BoxFuture<'static, Bf4Result<()>> + Send + 'static,
//     ) -> Bf4Result<()>
//     where
//         Self: Sized,
//     {
//         match ev.clone() {
//             Event::Chat { vis, player, msg } => {}
//             Event::RoundOver { winning_team } => {
//                 let winner = self.compute_result().await;
//                 bf4.say(
//                     format!("{} won the mapvote!", winner.unwrap()),
//                     Visibility::All,
//                 )
//                 .await
//                 .unwrap();

//                 // TODO actually switch to the new map
//             }
//             _ => {} // ignore other stuff
//         }

//         inner(bf4, ev).await
//     }
// }
