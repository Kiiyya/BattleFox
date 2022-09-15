use std::time::Duration;
use std::{collections::HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use battlefield_rcon::bf4::Visibility;
use battlefield_rcon::{bf4::{Bf4Client, Event}, rcon::RconResult};
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use battlelog::{get_loadout, get_users, search_user};

use crate::Plugin;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    enabled: bool,
    banned_weapons: HashMap<String, String>,
}

struct Inner {
    persona_ids: HashMap<String, String>
}

pub struct LoadoutEnforcer {
    config: Config,

    inner: RwLock<Inner>, // no Arc here, because we pass around `Arc<LoadoutEnforcer>`, so it would be redundant.
}

impl LoadoutEnforcer {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            inner: RwLock::new(Inner {
                persona_ids: HashMap::new(),
            }),
        }
    }

    async fn update_players(self: &Arc<Self>, bf4: &Arc<Bf4Client>) {
        trace!("[{}] Updating players/personaId map.", Self::NAME);

        let players = match bf4.list_players(Visibility::All).await {
            Ok(it) => it,
            Err(err) => {
                error!("[{}] Fetching players via RCON failed: {:?}.", Self::NAME, err);
                return;
            },
        };

        // Get just the player names
        let player_names: Vec<_> = players.iter()
            .map(|val| val.player_name.to_string())
            .collect();

        // Check for missing players
        let missing_players = {
            let rlock = self.inner.read();
            let mut missing_players: Vec<String> = Vec::new();
            for player_name in player_names.iter() {
                if !rlock.persona_ids.contains_key(player_name) {
                    missing_players.push(player_name.to_string());
                }
            }
            missing_players
        };

        // Insert missing player personaIds
        if !missing_players.is_empty() {
            match get_users(missing_players).await {
                Ok(users) => {
                    let mut wlock = self.inner.write();
                    for user in users.iter() {
                        trace!("[{}] Inserting personaId {} for {}", Self::NAME, user.persona.persona_id, user.persona.persona_name);
                        wlock.persona_ids.insert(user.persona.persona_name.to_string(), user.persona_id.to_string());
                    }
                },
                Err(err) => {
                    error!("[{}] Fetching personaIds from Battlelog failed: {:?}.", Self::NAME, err);
                    return;
                },
            };
        }

        // Remove players that have left
        self.inner.write().persona_ids.retain(|key, _value| {
            player_names.contains(key)
        });
    }

    async fn add_player(self: &Arc<Self>, player: &str) {
        trace!("[{}] Adding player {} to player/personaId map.", Self::NAME, player);

        // Insert if personaId missing
        if !self.inner.read().persona_ids.contains_key(player) {
            match search_user(player).await {
                Ok(user) => {
                    trace!("[{}] Inserting personaId {} for {}", Self::NAME, user.persona_id, user.persona_name);
                    self.inner.write().persona_ids.insert(user.persona_name.to_string(), user.persona_id.to_string());
                },
                Err(err) => {
                    error!("[{}] Fetching personaId from Battlelog failed: {:?}.", Self::NAME, err);
                },
            };
        }
    }

    async fn remove_player(self: &Arc<Self>, player: &str) {
        trace!("[{}] Removing player {} from the player/personaId map.", Self::NAME, player);

        // Insert if personaId missing
        let mut lock = self.inner.write();
        if lock.persona_ids.contains_key(player) {
            lock.persona_ids.remove(player);
        }
    }
}

#[async_trait]
impl Plugin for LoadoutEnforcer {
    const NAME: &'static str = "loadoutenforcer";

    fn enabled(&self) -> bool {
        self.config.enabled
    }

    async fn start(self: &Arc<Self>, bf4: &Arc<Bf4Client>) {
        self.update_players(bf4).await;
    }

    async fn event(self: Arc<Self>, bf4: Arc<Bf4Client>, ev: Event) -> RconResult<()> {
        match ev {
            Event::Authenticated { player } => {
                trace!("[{}] Authenticated - {}", Self::NAME, player.name);
                self.add_player(player.name.as_ref()).await;
            },
            Event::LevelLoaded { level_name, game_mode: _, rounds_played: _, rounds_total: _ } => {
                trace!("[{}] LevelLoaded - {}", Self::NAME, level_name.Pretty());
                self.update_players(&bf4).await;
            },
            Event::Leave { player, .. } => {
                trace!("[{}] Leave - {}", Self::NAME, player.name);
                self.remove_player(player.name.as_ref()).await;
            },
            Event::Disconnect { player, reason } => {
                trace!("[{}] Disconnect - {} > {}", Self::NAME, player, reason);
                self.remove_player(player.as_ref()).await;
            },
            Event::Spawn { player, .. } => {
                trace!("[{}] Spawn - {}", Self::NAME, player.name);

                let player_name = player.name.to_string();
                if !self.inner.read().persona_ids.contains_key(&player_name) {
                    warn!("[{}] Player ({}) doesn't have a known personaId, trying to fetch again...", Self::NAME, player.name);
                    self.add_player(player.name.as_ref()).await;
                }

                // let persona = persona_ids
                //     .get(&player_name)
                //     .map(|name: &String| (player_name.as_ref(), name.as_ref()))
                //     .unwrap_or(("unknown", "0"));

                // let persona_ids_clone = persona_ids.clone();
                // let self_clone = self.clone();
                // let bf4_clone = bf4.clone();
                let soldier_name: String;
                let persona_id: String;

                {
                    let read_lock = self.inner.read();
                    let persona = match read_lock.persona_ids.get_key_value(&player_name) {
                        Some(kvp) => kvp,
                        None => {
                            warn!("[{}] Player {} doesn't have a known personaId.", Self::NAME, player_name);
                            return Ok(()); // TODO: maybe throw an error instead? It won't crash, but it'll get logged.
                        },
                    };

                    // let persona = read_lock
                    //     .get(&player_name)
                    //     .map(|name: &String| (player_name.as_ref(), name.as_ref()))
                    //     .unwrap_or(("unknown", "0"));

                    soldier_name = persona.0.to_string();
                    persona_id = persona.1.to_string();
                }

                // Wait 5 seconds after spawn to try and make sure Battlelog has the updated loadout
                tokio::time::sleep(Duration::from_secs(5)).await;

                match get_loadout(&soldier_name, &persona_id).await {
                    Ok(loadout) => {
                        if loadout.current_loadout.is_none() {
                            trace!("[{}] {} > Loadout response didn't have current_loadout", Self::NAME, player.name);
                            return Ok(());
                        }

                        let current_loadout = loadout.current_loadout.unwrap();
                        let selected_kit = current_loadout.selected_kit.parse::<usize>().unwrap_or(99);
                        if selected_kit < 4 {
                            let weapon_codes = &current_loadout.kits[selected_kit];
                            trace!("[{}] {} > Weapon codes: {:?}", Self::NAME, player.name, weapon_codes);
                            for weapon_code in weapon_codes.iter() {
                                if self.config.banned_weapons.contains_key(weapon_code) {
                                    let kill_message = &self.config.banned_weapons[weapon_code];

                                    let _ = dbg!(bf4.kill(player.name.clone()).await);
                                    let _ = bf4.say(kill_message.to_string(), player.clone()).await;
                                    let _ = bf4.yell_dur(kill_message.to_string(), player.clone(), "10").await;

                                    trace!("[{}] {} > {}", Self::NAME, player.name, kill_message);
                                    return Ok(());
                                }
                            }

                            trace!("[{}] {} > Loadout was OK", Self::NAME, player.name);
                        }
                        else {
                            warn!("[{}] {} > Selected kit {} was invalid", Self::NAME, player.name, selected_kit);
                        }
                    },
                    Err(err) => {
                        error!("[{}] Fetching loadout from Battlelog failed: {:?}.", Self::NAME, err);
                    },
                };
            },
            // Ok(Event::RoundOver { winning_team: _ }) => {
            //     persona_ids.clear();
            // }
            _ => ()
        }

        Ok(())
    }
}