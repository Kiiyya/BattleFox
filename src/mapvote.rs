#![allow(unused_variables, unused_imports)]

use crate::{
    mapmanager::{
        pool::{MapInPool, MapPool, Vehicles, VehiclesSpecified},
        CallbackResult, MapManager, PopState,
    },
    stv::Profile,
};

use super::stv::tracing::NoTracer;
use super::stv::Ballot;
use ascii::AsciiString;
use battlefield_rcon::{
    bf4::{
        error::{Bf4Error, Bf4Result},
        Bf4Client, Event, GameMode, Map, Player, Visibility,
    },
    rcon::{RconError, RconResult},
};
use futures::StreamExt;
use std::fmt::Debug;
use std::hash::Hash;
use std::{
    any::Any,
    collections::{HashMap, HashSet},
    fmt::Display,
    future::Future,
    pin::Pin,
    sync::{Arc, Weak},
    time::Duration,
};
use tokio::{
    sync::Mutex,
    time::{sleep, Interval},
};

use num_rational::BigRational as Rat;
use num_traits::One;

#[derive(Debug)]
struct MapvoteInner {
    alternatives: HashSet<MapInPool<()>>,
    votes: HashMap<Player, Ballot<MapInPool<()>>>,

    pop_state: PopState<Vehicles>,
}

#[derive(Debug)]
pub struct Mapvote {
    inner: Mutex<MapvoteInner>,
    mapman: Arc<MapManager>,
}

impl MapvoteInner {
    pub fn to_profile(&self) -> Profile<MapInPool<()>> {
        Profile {
            alts: self.alternatives.clone(),
            ballots: self.votes.values().cloned().collect(),
        }
    }
}

/// When a user votes, they can still fuck up :)
#[derive(Debug, Clone)]
enum VoteResult<E: Eq + Clone> {
    Ok {
        new: Ballot<MapInPool<E>>,
        old: Option<Ballot<MapInPool<E>>>,
    },

    /// User submitted duplicate votes, but they were continuously together, and thus could be
    /// contracted into one. Emit warning, but accept vote.
    OkDuplicateButRemoved {
        new: Ballot<MapInPool<E>>,
        old: Option<Ballot<MapInPool<E>>>,
        duplicates: HashSet<Map>,
    },

    /// User submitted duplicates but they could not be untangled. Need to retry.
    ErrDuplicate { duplicates: HashSet<Map> },

    /// A map which the user voted on is not in the current map pool.
    /// The API can choose to nominate the map and then re-call `vote()`, or notify user, etc...
    ErrMapNotInPool { missing: HashSet<Map> },
}

impl Mapvote {
    /// Creates a new instance of `MapVote`, but doesn't start it yet, just sets stuff up.
    pub async fn new(mapman: Arc<MapManager>) -> Arc<Self> {
        let myself = Arc::new(Self {
            inner: Mutex::new(MapvoteInner {
                alternatives: HashSet::new(),
                votes: HashMap::new(),
                pop_state: mapman.pop_state().await,
            }),
            mapman: mapman.clone(),
        });

        // holy shit this is ugly.
        let myself_weak = Arc::downgrade(&myself);
        mapman
            .register_pool_change_callback(move |bf4, popstate| {
                let weak = myself_weak.clone();
                Box::pin(async move {
                    // try to upgrade to strong Arc<MapVote>. If that fails, it means the mapvote
                    // instance was dropped. In that case, RemoveMe.
                    if let Some(strong) = weak.upgrade() {
                        tokio::spawn(async move {
                            strong.on_popstate_changed(bf4, popstate).await;
                        });
                        CallbackResult::KeepGoing
                    } else {
                        CallbackResult::RemoveMe
                    }
                })
            })
            .await;

        myself
    }

    async fn on_popstate_changed(&self, bf4: Arc<Bf4Client>, popstate: PopState<Vehicles>) {
        println!("Popstate changed! New: {}", popstate.name);
        let mut lock = self.inner.lock().await;

        // TODO: check alternatives, we may need to remove some & notify people who voted for them.
        // TODO: Notify everyone of pop state change

        let removals = MapPool::removals(&lock.pop_state.pool, &popstate.pool);

        lock.pop_state = popstate;
        drop(lock);
        todo!("Handle popstate changes");
    }

    /// Starts the main loop, listening for events, etc.
    pub async fn run(self: Arc<Self>, bf4: Arc<Bf4Client>) -> RconResult<()> {
        let jh1 = {
            let mapvote = self.clone();
            let bf4 = bf4.clone();
            tokio::spawn(async move {
                mapvote.spam_status(bf4).await;
                println!("mapvote spammer sutatus done");
            })
        };

        let jh2 = {
            let mapvote = self.clone();
            let bf4 = bf4.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(313)).await;
                mapvote.spam_voting_guide(bf4).await;
                println!("mapvote spammer voting guide done");
            })
        };

        let mut events = bf4.event_stream().await?;
        while let Some(event) = events.next().await {
            match event {
                Ok(Event::Chat { vis, player, msg }) => {
                    let bf4 = bf4.clone();
                    let mapvote = self.clone();

                    if msg.as_str().starts_with("/haha next map") {
                        let mapman = self.mapman.clone();
                        tokio::spawn(async move {
                            mapvote.handle_round_over(&bf4, &mapman).await;
                        });
                    } else {
                        tokio::spawn(async move {
                            mapvote.handle_chat_msg(bf4, vis, player, msg).await;
                        });
                    }
                }
                Ok(Event::RoundOver { winning_team: _ }) => {
                    let bf4 = bf4.clone();
                    let mapvote = self.clone();
                    let mapman = self.mapman.clone();
                    // fire and forget about it, so we don't block other events. Yay concurrency!
                    tokio::spawn(async move {
                        // let's wait like 10 seconds because people might still vote in the end screen.
                        tokio::time::sleep(Duration::from_secs(12)).await;

                        mapvote.handle_round_over(&bf4, &mapman).await;
                    });
                }
                Err(Bf4Error::Rcon(RconError::ConnectionClosed)) => break,
                _ => {} // ignore everything else.
            }
        }

        jh1.await.unwrap();
        jh2.await.unwrap();
        Ok(())
    }

    async fn spam_status(&self, bf4: Arc<Bf4Client>) {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let lock = self.inner.lock().await;
            let profile = lock.to_profile();
            drop(lock); // drop lock before we spend 33ms in rcon call.

            let mut msg = vec!["========================================".to_string()];
            if let Some((winner, runnerup)) =
                dbg!(&profile).vanilla_stv_1_with_runnerup(&mut NoTracer)
            {
                msg.push(format!(
                    "[[MAPVOTE]] {} is in the lead! Vote now!",
                    winner.map.Pretty()
                ));

                if let Some(runner_up) = runnerup {
                    msg.push(format!("Current runner-up: {}", runner_up.map.Pretty()));
                }
            } else {
                msg.push("[[MAPVOTE]] No map has been voted for yet!".to_string());
            }
            msg.push("You can vote first, second, third, preferences like this:".to_string());
            msg.push("!metro oman_gulf pearlmarket".to_string());

            // msg.push("We use a fancy map vote (STV) here :)".to_string());
            // msg.push("You can vote first, second, third, preferences like this:".to_string());
            // msg.push("!metro gulf-of-oman pearlmarket".to_string());
            bf4.say_lines(msg, Visibility::All).await.unwrap();
        }
    }

    async fn spam_voting_guide(&self, bf4: Arc<Bf4Client>) {
        loop {
            tokio::time::sleep(Duration::from_secs(603)).await;
            // let mut msg = Vec::new();
            // msg.push("You can vote first, second, third, preferences like this:".to_string());
            // msg.push("!metro gulf-of-oman pearlmarket".to_string());
            let _ = bf4
                .say_lines(
                    vec![
                "We use a fancy voting rule here: Single Transferable Vote (STV) :)",
                // "You vote will not be spoiled, vote your conscience!",
                "If your first preference doesn't win, it gets transfered to 2nd, 3rd, etc..",
            ],
                    Visibility::All,
                )
                .await;
        }
    }

    /// Returns:
    /// - Ok:
    ///   - The ballot that ended up as the current vote.
    ///   - Optionally, old ballot of the previous time the player voted.
    /// - Err:
    ///   - Player did a derp.
    async fn vote(&self, player: &Player, alts: &[(Map, GameMode)]) -> VoteResult<()> {
        let ballot: Ballot<MapInPool<()>> = Ballot {
            weight: Rat::one(),
            preferences: alts
                .iter()
                .map(|(map, mode)| MapInPool {
                    map: *map,
                    mode: mode.clone(),
                    extra: (),
                })
                .collect(),
        };

        let mut lock = self.inner.lock().await;

        // TODO: make it so that only VIPs can nominate maps.
        ballot.preferences.iter().for_each(|pref| {
            // insert them all into the set. Set will dedup for us.
            let _ = lock.alternatives.insert(pref.clone());
        });

        let old = lock.votes.insert(player.clone(), ballot.clone());

        VoteResult::Ok { new: ballot, old }
    }

    async fn handle_chat_msg(
        &self,
        bf4: Arc<Bf4Client>,
        vis: Visibility,
        player: Player,
        msg: AsciiString,
    ) {
        let split = msg.as_str().split(' ').collect::<Vec<_>>();
        match split[0] {
            "/v" | "!v" => {
                let mut msg = Vec::new();
                let lock = self.inner.lock().await;
                if let Some(vote) = lock.votes.get(&player) {
                    // if player has already voted, just print that and nothing else.
                    msg.push(format!("You voted for {}", vote));
                } else {
                    // otherwise, print instructions on how to vote.
                    msg.push(
                        "You can vote first, second, etc... preferences like this:".to_string(),
                    );
                    msg.push("!metro gulf-of-oman pearlmarket".to_string());
                    msg.push("You haven't voted yet! Vote for ANY map you like".to_string());
                }

                drop(lock);
                let _ = bf4.say_lines(msg, player).await;
                return;
            }
            _ => {
                // if no command matched, try parsing !metro pearl etc
                match parse_maps(&msg.as_str()[1..]) {
                    ParseMapsResult::Ok(maps) => {
                        match self.vote(&player, &maps).await {
                            // Ok((vote, old_vote)) => {
                            //     // then just player feedback
                            // }
                            // Err(err) => {
                            //     // TODO replace with something better.
                            //     let _ = bf4
                            //         .say(format!("You did a whoopsie: {:?}", err), player)
                            //         .await;
                            // }
                            VoteResult::Ok { new, old } => {
                                if maps.len() == 1 {
                                    let _ = bf4.say_lines(vec![
                                        format!("You voted for {}, BUT you can specify a second, third,... preference on this server!", &maps[0].0.Pretty()),
                                        format!("Try it like this: !{} metro gulf-of-oman", &maps[0].0.short()),
                                    ], player).await;
                                } else {
                                    let _ = bf4
                                        .say(
                                            format!("{} voted for {}", player, new),
                                            Visibility::All,
                                        )
                                        .await;
                                    let _ = bf4.say("(You changed your vote)", player).await;
                                }
                            }
                            VoteResult::OkDuplicateButRemoved {
                                new,
                                old,
                                duplicates,
                            } => todo!("handle duplicates"),
                            VoteResult::ErrDuplicate { duplicates } => {
                                todo!("handle bad duplicates")
                            }
                            VoteResult::ErrMapNotInPool { missing } => {
                                todo!("handle map not in pool")
                            }
                        }
                    }
                    ParseMapsResult::Nothing => {}
                    ParseMapsResult::NotAMapName { orig } => {
                        let _ = bf4
                            .say(
                                format!("{}: \"{}\" is not a valid map name.", player, orig),
                                player,
                            )
                            .await;
                    }
                }
            }
        }
    }

    async fn handle_round_over(&self, bf4: &Arc<Bf4Client>, maplist: &Arc<MapManager>) {
        let profile = {
            let mut lock = self.inner.lock().await;
            let ret = lock.to_profile();
            lock.votes.clear();
            lock.alternatives.clear();
            ret
        };

        if let Some(mip) = profile.vanilla_stv_1(&mut NoTracer) {
            bf4.say(
                format!("[[MAPVOTE]] Winner: {:?}", mip.map),
                Visibility::All,
            )
            .await
            .unwrap();
            maplist.switch_to(bf4, &mip).await.unwrap();
            // maplist.switch_to(bf4, mipmap, mode, false).await.unwrap();
        } else {
            bf4.say("Round over, no winner", Visibility::All)
                .await
                .unwrap(); // TODO!!
        }
    }
}

pub enum ParseMapsResult {
    Ok(Vec<(Map, GameMode)>),

    /// Nothing, silently fail. E.g. when someone entered a normal command and not a map name.
    /// Returned when the first map name wasn't exact.
    Nothing,
    NotAMapName {
        orig: String, /*suggestions: Vec<(AsciiString, f64)> */
    },
}

/// expects a space-delimited list of maps with optional gamemode specifiers
///
/// The first map name must be exact, after that it'll trigger and give proper error messages.
/// If the first map is not an exact map name, it will just return `Nothing`.
pub fn parse_maps(str: &str) -> ParseMapsResult {
    let mut res = Vec::new();
    let words = str.split(' ').collect::<Vec<_>>();

    #[allow(clippy::needless_range_loop)]
    for i in 0..words.len() {
        // TODO: Add map@mode or map/mode or map:mode syntax
        if let Some(map) = Map::try_from_short(words[i]) {
            res.push((map, GameMode::Rush));
        } else if i == 0 {
            return ParseMapsResult::Nothing;
        } else {
            return ParseMapsResult::NotAMapName {
                orig: words[i].to_owned(),
            };
        }
    }

    ParseMapsResult::Ok(res)
}
