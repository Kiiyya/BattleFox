#![allow(unused_variables, unused_imports)]

use crate::{
    mapmanager::{MapInPool, MapManager, MapPool},
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

/// An alternative. As in, one thing you can vote on.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Alt(Map, GameMode);

impl Display for Alt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.Pretty())
    }
}

#[derive(Debug)]
struct MapvoteInner {
    alternatives: HashSet<Alt>,
    votes: HashMap<Player, Ballot<Alt>>,
}

#[derive(Debug)]
pub struct Mapvote {
    inner: Mutex<MapvoteInner>,
    mapman: Arc<MapManager>,
}

impl MapvoteInner {
    pub fn to_profile(&self) -> Profile<Alt> {
        Profile {
            alts: self.alternatives.clone(),
            ballots: self.votes.values().cloned().collect(),
        }
    }
}

/// When a user votes, they can still fuck up :)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum VoteResult {
    /// User voted for the first time.
    Ok,

    /// Used changed their vote, previous they had voted for...
    OkChangedVote(Alt),

    /// User submitted duplicate votes, but they were continuously together, and thus could be
    /// contracted into one. Emit warning, but accept vote.
    DuplicateButRemoved(Alt),

    /// User submitted duplicates but they could not be untangled. Need to retry.
    Duplicate(Alt),

    /// A map which the user voted on is not in the current map pool.
    /// The API can choose to nominate the map and then re-call `vote()`, or notify user, etc...
    MapNotInPool(Alt),
}


impl Mapvote {
    /// Creates a new instance of `MapVote`, but doesn't start it yet, just sets stuff up.
    pub fn new(mapman: Arc<MapManager>) -> Self {
        Self {
            inner: Mutex::new(MapvoteInner {
                alternatives: HashSet::new(),
                votes: HashMap::new(),
            }),
            mapman,
        }
        // TODO: set up callbacks n stuff.
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
                    // fire and forget about it, so we don't block other events. Yay concurrency!

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

    pub async fn spam_status(&self, bf4: Arc<Bf4Client>) {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let lock = self.inner.lock().await;
            let profile = lock.to_profile();
            drop(lock); // drop lock before we spend 33ms in rcon call.

            let mut msg = vec!["========================================".to_string()];
            if let Some((winner, runnerup)) =
                dbg!(&profile).vanilla_stv_1_with_runnerup(&mut NoTracer)
            {
                msg.push(format!("[[MAPVOTE]] {} is in the lead! Vote now!", winner));

                if let Some(runner_up) = runnerup {
                    msg.push(format!("Current runner-up: {}", runner_up));
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

    pub async fn spam_voting_guide(&self, bf4: Arc<Bf4Client>) {
        loop {
            tokio::time::sleep(Duration::from_secs(313)).await;
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

    /// returns Some(old_ballot) if player had voted before.
    pub async fn vote(&self, player: &Player, alts: &[Alt]) -> Option<Ballot<Alt>> {
        let ballot = Ballot {
            weight: Rat::one(),
            preferences: alts.to_owned(),
        };

        let mut lock = self.inner.lock().await;
        alts.iter().for_each(|alt| {
            let _ = lock.alternatives.insert(alt.to_owned());
        });
        lock.votes.insert(player.clone(), ballot)
    }

    pub async fn handle_chat_msg(
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
                        self.vote(&player, &maps).await; // <-- (vote & lock happens in here)

                        // then just player feedback
                        if maps.len() == 1 {
                            let _ = bf4.say_lines(vec![
                                format!("You voted for {}, BUT you can specify a second, third,... preference on this server!", &maps[0].0.Pretty()),
                                format!("Try it like this: !{} metro gulf-of-oman", &maps[0].0.short()),
                            ], Visibility::All).await;
                        } else {
                            let _ = bf4
                                .say(
                                    format!(
                                        "Your first preference is {}, second {}, etc.",
                                        &maps[0].0.Pretty(), &maps[1].0.Pretty()
                                    ),
                                    Visibility::All,
                                )
                                .await;
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

    // pub async fn map_pool_changed(new: &MapPool) -> RconResult<()> {

    // }

    pub async fn handle_round_over(&self, bf4: &Arc<Bf4Client>, maplist: &Arc<MapManager>) {
        let profile = {
            let mut lock = self.inner.lock().await;
            let ret = lock.to_profile();
            lock.votes.clear();
            lock.alternatives.clear();
            ret
        };

        if let Some(Alt(map, mode)) = profile.vanilla_stv_1(&mut NoTracer) {
            bf4.say(format!("[[MAPVOTE]] Winner: {:?}", map), Visibility::All)
                .await
                .unwrap();
            maplist.switch_to(bf4, map, mode, false).await.unwrap();
        } else {
            bf4.say("Round over, no winner", Visibility::All)
                .await
                .unwrap(); // TODO!!
        }
    }
}

pub enum ParseMapsResult {
    Ok(Vec<Alt>),

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
            res.push(Alt(map.clone(), GameMode::Rush));
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
