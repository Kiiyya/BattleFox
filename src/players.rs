//! Keeps track all players currently on the server.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use battlefield_rcon::{
    bf4::{Bf4Client, Event, Player, Squad, Team, Visibility},
    rcon::RconResult,
};
use futures::StreamExt;
use tokio::sync::Mutex;

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

#[derive(Debug)]
struct Inner {
    players: HashMap<Player, PlayerInServer>,
    players_joining: HashMap<Player, PlayerJoining>,
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
            }),
        }
    }

    pub async fn players(&self) -> HashMap<Player, PlayerInServer> {
        let inner = self.inner.lock().await;
        inner.players.clone()
    }

    pub async fn poller(&self, bf4: Arc<Bf4Client>) -> RconResult<()> {
        loop {
            let list = bf4.list_players(Visibility::All).await.unwrap(); // TODO: unwrap
            let now = Instant::now();
            {
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
                // drop lock here, before timer.
            }

            tokio::time::sleep(Duration::from_secs(60 * 3 + 18)).await;
        }
    }

    pub async fn run(self: Arc<Self>, bf4: Arc<Bf4Client>) -> RconResult<()> {
        tokio::spawn({
            let bf4 = Arc::clone(&bf4);
            let myself = Arc::clone(&self);
            async move {
                myself.poller(bf4).await
            }
        });

        let mut events = bf4.event_stream().await?;
        while let Some(event) = events.next().await {
            let now = Instant::now();
            match event {
                Ok(Event::Authenticated { player }) => {
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
                }
                Ok(Event::Leave {
                    player,
                    final_scores: _,
                }) => {
                    let mut inner = self.inner.lock().await;
                    inner.players_joining.remove(&player);
                    inner.players.remove(&player);
                }
                Ok(Event::SquadChange {
                    player,
                    team,
                    squad,
                }) => {
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
                }
                Ok(Event::TeamChange {
                    player,
                    team,
                    squad,
                }) => {
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
                }
                Ok(Event::Spawn { player, team }) => {
                    let mut inner = self.inner.lock().await;
                    if let Some(p) = inner.players.get_mut(&player) {
                        p.team = team;
                        p.last_seen = now;
                    }
                    if let Some(p) = inner.players_joining.get_mut(&player) {
                        p.team = Some(team);
                        p.last_seen = now;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
