use std::{collections::{HashMap}, convert::TryInto, sync::{Arc}};

use ascii::AsciiString;
use battlefield_rcon::{bf4::{Bf4Client, Eaid, Event, CommmoRose}, rcon::RconResult};
use database::{delete_muted_player, establish_connection, get_muted_player, get_muted_players, replace_into_muted_player};
use futures::StreamExt;
use serde::{Serialize, Deserialize};
use shared::{events::{PlayerMuted, PlayerUnmuted}, mute::MuteType};
use tokio::sync::Mutex;

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerMuteConfig {
    enabled: bool
}

pub struct PlayerMute {
    config: PlayerMuteConfig
}

struct MutedPlayerInfo {
    infractions: usize,
    mute_type: MuteType,
}

impl PlayerMute {
    pub fn new(config: PlayerMuteConfig) -> Arc<Self> {
        let myself = Arc::new(Self {
            config
        });

        myself
    }

    pub async fn run(self: Arc<Self>, bf4: Arc<Bf4Client>) -> RconResult<()> {
        if !self.config.enabled {
            debug!("Player muting is disabled");
            return Ok(());
        }

        debug!("Starting player mute addon");

        let offenses: Arc<Mutex<HashMap<Eaid, MutedPlayerInfo>>> = Arc::new(Mutex::new(HashMap::new()));

        // Fetch muted players
        {
            let offenses = Arc::clone(&offenses);
            let mut offenses = offenses
                .lock()
                .await;
            self.update_players(&mut offenses);
        }

        let mut stream = bf4.event_stream().await?;
        while let Some(event) = stream.next().await {
            match event {
                Ok(Event::LevelLoaded { level_name, game_mode, rounds_played, rounds_total}) => {
                    debug!("Player Mute - Level loaded: {} {} {} {}", level_name.Pretty(), game_mode, rounds_played, rounds_total);

                    let offenses = Arc::clone(&offenses);
                    let playerreport = self.clone();

                    // Update mute list (remove expired mutes and add missing ones)
                    tokio::spawn(async move {
                        let mut offenses = offenses
                            .lock()
                            .await;

                        playerreport.update_players(&mut offenses);
                    });
                },
                // Ok(Event::ServerChat { vis, msg }) => {
                //     debug!("Server > {}", msg);

                //     let split = msg.as_str()
                //         .split(' ')
                //         .filter(|&s| !s.is_empty())
                //         .collect::<Vec<_>>();

                //     if split.len() < 2 {
                //         continue;
                //     }

                //     let player = match bf4.resolve_player(&AsciiString::from_ascii(split[0]).unwrap()).await {
                //         Ok(player) => player,
                //         _ => {
                //             warn!("Player {} not found", split[0]);
                //             continue;
                //         }
                //     };
                //     let message = AsciiString::from_ascii(split[1..].join(" ")).unwrap();

                //     let bf4 = bf4.clone();
                //     let playerreport = self.clone();

                //     tokio::spawn(async move {
                //         if let Err(_) = CommmoRose::decode(&message) {
                //             // Not a commo rose message

                //         }
                //     });
                // },
                Ok(Event::Chat { vis: _, player, msg }) => {
                    debug!("{} > {}", player.name, msg);

                    if msg.as_str().starts_with('/') { continue; }
                    if CommmoRose::decode(&msg).is_ok() { continue; }

                    let mut lock = offenses.lock().await;
                    if let Some(muted_player) = lock.get_mut(&player.eaid) {
                        muted_player.infractions += 1;
                        let infractions = muted_player.infractions;
                        drop(lock);

                        let bf4 = bf4.clone();
                        let self_clone = self.clone();
                        tokio::spawn(async move {
                            if infractions >= 2 {
                                let _ = dbg!(bf4.kick(player.name.clone(), "You have been kicked for talking while muted.").await);
                                self_clone.add_kicked(&player.eaid); // FIXME: this makes several synchronous blocking SQL / Diesel calls!
                            } else {
                                let _ = dbg!(bf4.kill(player.name.clone()).await);
                                let _ = bf4.say("You are muted and are not allowed to talk in the server. You'll be kicked next time.", player.clone()).await;
                            }
                        });
                    }
                },
                Ok(Event::RoundOver { winning_team }) => {
                    debug!("Player Mute - Round Over: {:#?}", winning_team);

                    let offenses = Arc::clone(&offenses);
                    let playerreport = self.clone();

                    // Remove all that only had round mute
                    tokio::spawn(async move {
                        let mut offenses = offenses
                            .lock()
                            .await;

                        playerreport.remove_round_mutes(&mut offenses);
                    });
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn update_players(self: &Self, map: &mut HashMap<Eaid, MutedPlayerInfo>) {
        debug!("Updating muted players");

        match establish_connection() {
            Ok(con) => {
                let result = get_muted_players(&con);
                match result {
                    Ok(muted_players) => {
                        debug!("Muted players: {:#?}", muted_players);

                        // Remove people who have gotten unmuted/mute has expired
                        map.retain(|&key, _| muted_players.iter().any(|p| key.to_string() == p.eaid));

                        // Add missing muted people
                        for muted_player in muted_players.iter() {
                            let eaid = Eaid::new(&AsciiString::from_ascii(muted_player.eaid.clone()).unwrap());
                            if let Ok(eaid) = eaid {
                                map.entry(eaid).or_insert(MutedPlayerInfo {
                                    infractions: 0,
                                    mute_type: muted_player.type_.try_into().unwrap()
                                });

                                debug!("Added or updated mute for: {:#?}", eaid);
                            }
                        }
                    }
                    Err(err) => error!("Error fetching muted players: {}", err),
                }
            },
            Err(error) => error!("Failed to connect to database: {}", error),
        }
    }

    fn remove_round_mutes(self: &Self, map: &mut HashMap<Eaid, MutedPlayerInfo>) {
        debug!("Removing round muted players");

        match establish_connection() {
            Ok(con) => {

                for (key, val) in map.iter() {
                    if val.mute_type == MuteType::Round {
                        let result = delete_muted_player(&con, key.to_string());
                        match result {
                            Ok(_) => debug!("Removed mute from: {:#?}", key),
                            Err(err) => error!("Error trying to remove mute from {}: {}", key, err),
                        }
                    }
                }
            },
            Err(error) => error!("Failed to connect to database: {}", error),
        }
    }

    fn add_kicked(self: &Self, eaid: &Eaid) {
        debug!("Adding kick for {}", eaid.to_string());

        match establish_connection() {
            Ok(con) => {
                if let Ok(mut player) = get_muted_player(&con, &eaid.to_string()) {
                    player.kicks = Some(player.kicks.unwrap_or(0) + 1);

                    match replace_into_muted_player(&con, player) {
                        Ok(_) => debug!("Added kick for {}", eaid.to_string()),
                        Err(_) => debug!("Failed to add kick for {}", eaid.to_string()),
                    }
                }
            },
            Err(error) => error!("Failed to connect to database: {}", error),
        }
    }
}