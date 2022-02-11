//! Keeps track all players currently on the server.

use std::{collections::{BTreeMap, HashMap}, sync::Arc, time::{Duration, Instant}};

use async_trait::async_trait;
use battlefield_rcon::{
    bf4::{Bf4Client, Event, Player, Squad, Team, Visibility},
    rcon::RconResult,
};
use strsim::levenshtein;
use tokio::sync::Mutex;

use crate::Plugin;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct PlayerInServer {
    pub player: Player,
    pub team: Team,
    pub squad: Squad,
    pub last_seen: Instant,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct PlayerJoining {
    pub player: Player,
    pub team: Option<Team>,
    pub squad: Option<Squad>,
    pub last_seen: Instant,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum MatchError {
    NoMatches,
    TooMany,
}

#[derive(Debug)]
struct Inner {
    players: HashMap<Player, PlayerInServer>,
    players_joining: HashMap<Player, PlayerJoining>,

    last_checked: Option<Instant>,
}

impl Inner {
    /// attempts to move player from joining to normal.
    fn check_submit(&mut self, player: &Player) {
        let mut del = false;
        if let Some(p) = self.players_joining.get(player) {
            if let Some(team) = p.team {
                if let Some(squad) = p.squad {
                    self.players.insert(
                        player.to_owned(),
                        PlayerInServer {
                            player: player.to_owned(),
                            team,
                            squad,
                            last_seen: Instant::now(),
                        },
                    );
                    del = true;
                }
            }
        }

        if del {
            self.players_joining.remove(player);
        }
    }

    fn trim_old(&mut self) {
        self.players
            .retain(|_, v| v.last_seen.elapsed() < Duration::from_secs(60 * 10));
    }
}

#[derive(Debug)]
pub struct Players {
    inner: Mutex<Inner>,
}

impl Players {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                players: HashMap::new(),
                players_joining: HashMap::new(),
                last_checked: None,
            }),
        }
    }

    pub async fn player(&self, player: &Player) -> Option<PlayerInServer> {
        let inner = self.inner.lock().await;
        inner.players.get(player).cloned()
    }

    pub async fn players(&self, bf4: &Bf4Client) -> HashMap<Player, PlayerInServer> {
        let inner = self.inner.lock().await;
        if let Some(last_checked) = inner.last_checked {
            if last_checked.elapsed() < Duration::from_secs(60 * 3 + 18) {
                return inner.players.clone();
            }
        }

        drop(inner);
        self.refresh(bf4, |inner| inner.players.clone()).await
    }

    pub async fn poller(&self, bf4: Arc<Bf4Client>) -> RconResult<()> {
        loop {
            self.refresh(&bf4, |_| ()).await;
            tokio::time::sleep(Duration::from_secs(60 * 3 + 18)).await;
        }
    }

    async fn refresh<Ret>(&self, bf4: &Bf4Client, getter: impl FnOnce(&mut Inner) -> Ret) -> Ret {
        let list = bf4.list_players(Visibility::All).await.unwrap();
        let now = Instant::now();

        let mut inner = self.inner.lock().await;
        inner.players.clear(); // yeet it all.
        for pi in list {
            let player = Player {
                name: pi.player_name,
                eaid: pi.eaid,
            };
            inner.players.insert(
                player.clone(),
                PlayerInServer {
                    player,
                    team: pi.team,
                    squad: pi.squad,
                    last_seen: now,
                },
            );
        }
        inner.last_checked = Some(now);
        getter(&mut inner)
    }

    pub async fn get_best_player_match(&self, target: &str) -> Result<Player, MatchError> {
        let inner = self.inner.lock().await;
        // let mut current_players = inner.players.clone();

        let target_lowercase = target.to_ascii_lowercase();
        // Remove to allow errors in typing, for example 'I' as 'l'
        let current_players = inner.players
            .iter()
            .filter(|(k, _v)| k.name.to_ascii_lowercase().to_string().contains(&target_lowercase))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        drop(inner);

        //self.players_contains(&current_players, target); // Remove to allow errors in typing, for example 'I' as 'l'

        let players_start_with = self.players_starts_with(&current_players, target);
        if !players_start_with.is_empty() {
            let matches = self.get_levenshtein(&players_start_with, target);
            if matches.is_empty() {
                Err(MatchError::NoMatches)
            }
            else if matches.len() == 1 {
                Ok(matches.iter().next().unwrap().1.clone())
            }
            else {
                Err(MatchError::TooMany)
            }
        }
        else {
            let matches = self.get_levenshtein(&current_players, target);
            if matches.is_empty() {
                Err(MatchError::NoMatches)
            }
            else if matches.len() == 1 {
                Ok(matches.iter().next().unwrap().1.clone())
            }
            else {
                let mut iterator = matches.iter();

                let first = iterator.next().unwrap();
                let second = iterator.next().unwrap();

                if second.0 - first.0 > 2 {
                    Ok(first.1.clone())
                }
                else {
                    Err(MatchError::TooMany)
                }
            }
        }
    }

    // fn players_contains(&self, map: &mut HashMap<Player, PlayerInServer>, target: &str) {
    //     let target_lowercase = target.to_ascii_lowercase();
    //     map.retain(|key, _value| {
    //         key.name.to_ascii_lowercase().to_string().contains(&target_lowercase)
    //     })
    // }

    fn players_starts_with(&self, map: &HashMap<Player, PlayerInServer>, target: &str) -> HashMap<Player, PlayerInServer> {
        let target_lowercase = target.to_ascii_lowercase();
        map.iter().filter_map(|(key, value)| {
            if key.name.to_ascii_lowercase().to_string().starts_with(&target_lowercase) {
                Some((key.to_owned(), value.to_owned()))
            } else {
                None
            }
        }).collect()
    }

    fn get_levenshtein(&self, map: &HashMap<Player, PlayerInServer>, target: &str) -> BTreeMap<usize, Player> {
        let target_lowercase = target.to_ascii_lowercase();
        map.iter().map(|(key, _value)| {
            (levenshtein(&target_lowercase, &key.name.to_ascii_lowercase().to_string()), key.to_owned())
        }).collect()
    }
}

#[async_trait]
impl Plugin for Players {
    const NAME: &'static str = "players";

    async fn event(self: Arc<Self>, _bf4: Arc<Bf4Client>, event: Event) -> RconResult<()> {
        let now = Instant::now();
        match event {
            Event::Authenticated { player } => {
                let mut inner = self.inner.lock().await;
                inner.players_joining.insert(
                    player.clone(),
                    PlayerJoining {
                        player,
                        team: None,
                        squad: None,
                        last_seen: now,
                    },
                );
                inner.trim_old();
            },
            Event::Leave {
                player,
                final_scores: _,
            } => {
                let mut inner = self.inner.lock().await;
                inner.players_joining.remove(&player);
                inner.players.remove(&player);
            },
            Event::SquadChange {
                player,
                team,
                squad,
            } => {
                let mut inner = self.inner.lock().await;
                if let Some(p) = inner.players.get_mut(&player) {
                    p.team = team;
                    p.squad = squad;
                    p.last_seen = now;
                }
                if let Some(p) = inner.players_joining.get_mut(&player) {
                    p.team = Some(team);
                    p.squad = Some(squad);
                    p.last_seen = now;
                    inner.check_submit(&player);
                }
            },
            Event::TeamChange {
                player,
                team,
                squad,
            } => {
                let mut inner = self.inner.lock().await;
                if let Some(p) = inner.players.get_mut(&player) {
                    p.team = team;
                    p.squad = squad;
                    p.last_seen = now;
                }
                if let Some(p) = inner.players_joining.get_mut(&player) {
                    p.team = Some(team);
                    p.squad = Some(squad);
                    p.last_seen = now;
                    inner.check_submit(&player);
                }
            },
            Event::Spawn { player, team } => {
                let mut inner = self.inner.lock().await;
                if let Some(p) = inner.players.get_mut(&player) {
                    p.team = team;
                    p.last_seen = now;
                }
                if let Some(p) = inner.players_joining.get_mut(&player) {
                    p.team = Some(team);
                    p.last_seen = now;
                }
            },
            _ => (),
        }
        Ok(())
    }
}

