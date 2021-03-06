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

use num_rational::BigRational as Rat;
use num_traits::One;

/// An alternative. As in, one thing you can vote on.
/// This is a `(Map, GameMode)` tuple currently.
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
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(MapvoteInner {
                alternatives: HashSet::new(),
                votes: HashMap::new(),
            }),
        }
    }

    pub fn format_status(&self) -> Vec<String> {
        let mut ret = Vec::new();

        

        ret
    }

    // #[periodic(self.spam_interval)]
    // async fn spam_votes(&self) { ... }

    /// returns Some(old_ballot) if player had voted before.
    // #[command("vote", "v")]
    pub async fn vote(&self, player: &Player, alts: &[Alt]) -> Option<Ballot<Alt>> {
        let ballot = Ballot {
            weight: Rat::one(),
            preferences: alts.to_owned()
        };

        let mut lock = self.inner.lock().await;
        lock.votes.insert(player.clone(), ballot)
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
}

// impl ContributesPeriodic for Mapvote {
//      
// }
// 
// impl ContributesCommand for Mapvote {
//      eh no, this would mean it only contributes a single command, hm..
// }
// 


pub enum ParseMapsResult {
    Ok(Vec<Alt>),
    /// Nothing, silently fail. E.g. when someone entered a normal command and not a map name.
    /// Returned when the first map name wasn't exact.
    Nothing,
    NotAMapName { orig: String, /*suggestions: Vec<(AsciiString, f64)> */},
}

/// expects a space-delimited list of maps with optional gamemode specifiers
/// 
/// The first map name must be exact, after that it'll trigger and give proper error messages.
/// If the first map is not an exact map name, it will just return `Nothing`.
pub fn parse_maps(str: &str) -> ParseMapsResult {
    let mut res = Vec::new();
    let words = str.split(' ').collect::<Vec<_>>();

    for i in 0..words.len() {
        // TODO: Add map@mode or map/mode or map:mode syntax
        if let Some(map) = Map::try_from_short(words[i]) {
            res.push((map.clone(), GameMode::Rush));
        } else {
            if i == 0 {
                return ParseMapsResult::Nothing;
            } else {
                return ParseMapsResult::NotAMapName { orig: words[i].to_owned(), };
            }
        }
    }

    ParseMapsResult::Ok(res)
}
