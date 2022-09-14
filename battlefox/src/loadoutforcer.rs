use std::time::Duration;
use std::{collections::HashMap, hash::Hash};
use std::sync::Arc;

use async_trait::async_trait;
use battlefield_rcon::{bf4::{Bf4Client, Event, Player, Weapon, Visibility}, rcon::RconResult};
use futures::StreamExt;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use battlelog::{get_users, get_loadout, search_user};

use crate::Plugin;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    enabled: bool,
    banned_weapons: HashMap<String, String>,
}

pub struct LoadoutEnforcer {
    config: Config,
}

impl LoadoutEnforcer {
    pub fn new(config: Config) -> Self {
        Self {
            config
        }
    }

    async fn update_players(self: Arc<Self>, bf4: Arc<Bf4Client>, persona_ids: &mut Arc<RwLock<HashMap<String, String>>>) {
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
        let mut missing_players: Vec<String> = Vec::new();
        for player_name in player_names.iter() {
            if !persona_ids.read().contains_key(player_name) {
                missing_players.push(player_name.to_string());
            }
        }

        // Insert missing player personaIds
        if missing_players.len() > 0 {
            match get_users(missing_players).await {
                Ok(users) => {
                    for user in users.iter() {
                        trace!("[{}] Inserting personaId {} for {}", Self::NAME, user.persona.persona_id, user.persona.persona_name);
                        persona_ids.write().insert(user.persona.persona_name.to_string(), user.persona_id.to_string());
                    }
                },
                Err(err) => {
                    error!("[{}] Fetching personaIds from Battlelog failed: {:?}.", Self::NAME, err);
                    return;
                },
            };
        }

        // Remove players that have left
        persona_ids.write().retain(|key, _value| {
            player_names.contains(&key)
        });
    }

    async fn add_player(self: Arc<Self>, bf4: Arc<Bf4Client>, persona_ids: &mut Arc<RwLock<HashMap<String, String>>>, player: &str) {
        trace!("[{}] Adding player {} to player/personaId map.", Self::NAME, player);
        
        // Insert if personaId missing
        if !persona_ids.read().contains_key(player) {
            match search_user(player).await {
                Ok(user) => {
                    trace!("[{}] Inserting personaId {} for {}", Self::NAME, user.persona_id, user.persona_name);
                    persona_ids.write().insert(user.persona_name.to_string(), user.persona_id.to_string());
                },
                Err(err) => {
                    error!("[{}] Fetching personaId from Battlelog failed: {:?}.", Self::NAME, err);
                    return;
                },
            };
        }
    }

    async fn remove_player(self: Arc<Self>, bf4: Arc<Bf4Client>, persona_ids: &mut Arc<RwLock<HashMap<String, String>>>, player: &str) {
        trace!("[{}] Removing player {} from the player/personaId map.", Self::NAME, player);
        
        // Insert if personaId missing
        if persona_ids.read().contains_key(player) {
            persona_ids.write().remove(player);
        }
    }
}

#[async_trait]
impl Plugin for LoadoutEnforcer {
    const NAME: &'static str = "loadoutenforcer";

    fn enabled(&self) -> bool {
        self.config.enabled
    }

    async fn run(self: Arc<Self>, bf4: Arc<Bf4Client>) -> RconResult<()> {
        info!("Plugin {} is {}.", Self::NAME, if self.enabled() { "enabled" } else { "disabled" });
        if self.enabled() {
            let mut persona_ids : Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));
            self.clone().update_players(bf4.clone(), &mut persona_ids).await;
    
            let mut stream = bf4.event_stream().await?;
            while let Some(event) = stream.next().await {
                match event {
                    Ok(Event::Authenticated { player }) => {
                        trace!("[{}] Authenticated - {}", Self::NAME, player.name);
                        self.clone().add_player(bf4.clone(), &mut persona_ids, &player.name.to_string()).await;
                    },
                    Ok(Event::LevelLoaded { level_name, game_mode: _, rounds_played: _, rounds_total: _ }) => {
                        trace!("[{}] LevelLoaded - {}", Self::NAME, level_name.Pretty());
                        self.clone().update_players(bf4.clone(), &mut persona_ids).await;
                    },
                    Ok(Event::Leave { player, final_scores: _ }) => {
                        trace!("[{}] Leave - {}", Self::NAME, player.name);
                        self.clone().remove_player(bf4.clone(), &mut persona_ids, &player.name.to_string()).await;
                    },
                    Ok(Event::Disconnect { player, reason })=> {
                        trace!("[{}] Disconnect - {} > {}", Self::NAME, player, reason);
                        self.clone().remove_player(bf4.clone(), &mut persona_ids, &player.to_string()).await;
                    },
                    Ok(Event::Spawn { player, team: _ }) => {
                        trace!("[{}] Spawn - {}", Self::NAME, player.name);
    
                        let player_name = player.name.to_string();
                        if !persona_ids.read().contains_key(&player_name) {
                            warn!("[{}] Player ({}) doesn't have a known personaId, trying to fetch again...", Self::NAME, player.name);
                            self.clone().add_player(bf4.clone(), &mut persona_ids, &player.name.to_string()).await;
                            continue;
                        }
    
                        // let persona = persona_ids
                        //     .get(&player_name)
                        //     .map(|name: &String| (player_name.as_ref(), name.as_ref()))
                        //     .unwrap_or(("unknown", "0"));
    
                        let persona_ids_clone = persona_ids.clone();
                        let self_clone = self.clone();
                        let bf4_clone = bf4.clone();
                        tokio::spawn(async move {
                            let soldier_name: String;
                            let persona_id: String;
    
                            {
                                let read_lock = persona_ids_clone.read();
                                let persona = match read_lock.get_key_value(&player_name) {
                                    Some(kvp) => kvp,
                                    None => {
                                        warn!("[{}] Player {} doesn't have a known personaId.", Self::NAME, player_name);
                                        return;
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
                            match get_loadout(&soldier_name,&persona_id).await {
                                Ok(loadout) => {
                                    if let None = loadout.current_loadout { 
                                        trace!("[{}] {} > Loadout response didn't have current_loadout", Self::NAME, player.name);
                                        return;
                                    }

                                    let current_loadout = loadout.current_loadout.unwrap();
                                    let selected_kit = current_loadout.selected_kit.parse::<usize>().unwrap_or(99);
                                    if selected_kit < 4 {
                                        let weapon_codes = &current_loadout.kits[selected_kit];
                                        trace!("[{}] {} > Weapon codes: {:?}", Self::NAME, player.name, weapon_codes);
                                        for weapon_code in weapon_codes.iter() {
                                            if self_clone.config.banned_weapons.contains_key(weapon_code) {
                                                let kill_message = &self_clone.config.banned_weapons[weapon_code];
        
                                                let _ = dbg!(bf4_clone.kill(player.name.clone()).await);
                                                let _ = bf4_clone.say(format!("{}", kill_message), player.clone()).await;
                                                let _ = bf4_clone.yell(format!("{}", kill_message), None, player.clone()).await;
                                                trace!("[{}] {} > {}", Self::NAME, player.name, kill_message);
                                                return;
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
                        });
                    },
                    // Ok(Event::RoundOver { winning_team: _ }) => {
                    //     persona_ids.clear();
                    // }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}