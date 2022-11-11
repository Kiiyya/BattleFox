use std::{collections::{HashMap}, convert::TryInto, ops::Add, sync::{Arc}};

use ascii::AsciiString;
use async_trait::async_trait;
use battlefield_rcon::{bf4::{Bf4Client, CommmoRose, Eaid, Event, Player}, rcon::RconResult};
use battlefox_database::BfoxContext;
use battlefox_database::adkats::mutes::BfoxMutedPlayer;
use itertools::Itertools;
use serde::{Serialize, Deserialize};
use battlefox_shared::mute::MuteType;
use tokio::sync::Mutex;

use crate::Plugin;
use crate::players::{MatchError, Players};

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerMuteConfig {
    enabled: bool,
    mute_permissions: HashMap<AsciiString, bool>,
}

pub struct PlayerMute {
    players: Arc<Players>,
    offenses: Arc<Mutex<HashMap<Eaid, MutedPlayerInfo>>>,
    config: PlayerMuteConfig,
    db: BfoxContext,
}

struct MutedPlayerInfo {
    infractions: usize,
    mute_type: MuteType,
    #[allow(dead_code)]
    reason: Option<String>
}

#[async_trait]
impl Plugin for PlayerMute {
    const NAME: &'static str = "playermute";

    fn enabled(&self) -> bool { self.config.enabled }

    async fn event(self: Arc<Self>, bf4: Arc<Bf4Client>, event: Event) -> RconResult<()> {
        match event {
            Event::LevelLoaded { level_name, game_mode, rounds_played, rounds_total} => {
                // Update mute list (remove expired mutes and add missing ones)
                self.update_players();
            },
            Event::Chat { vis: _, player, msg } => {
                if msg.as_str().starts_with('/') { return Ok(()); }
                if CommmoRose::decode(&msg).is_ok() { return Ok(()); }

                let mut lock = self.offenses.lock().await;
                if let Some(muted_player) = lock.get_mut(&player.eaid) {
                    muted_player.infractions += 1;
                    let infractions = muted_player.infractions;
                    drop(lock);

                    let bf4 = bf4.clone();
                    if infractions >= 2 {
                        match bf4.kick(player.name.clone(), "You have been kicked for talking while muted.").await {
                            Ok(_) => {
                                self.add_kicked(&player.eaid).await;
                            },
                            Err(error) => {
                                let _ = dbg!(error);
                            }
                        };
                    } else {
                        let _ = dbg!(bf4.kill(player.name.clone()).await);
                        let _ = bf4.say("You are muted and are not allowed to talk in the server. You'll be kicked next time.", player.clone()).await;
                    }

                    // Mute/Unmute command handling
                    let _ = self.handle_chat_msg(bf4, player, msg).await;
                }
            },
            Event::RoundOver { winning_team } => {
                // Remove all that only had round mute
                self.remove_round_mutes().await;
            },
            _ => {}
        }
        Ok(())
    }
}


impl PlayerMute {
    pub fn new(db: BfoxContext, players: Arc<Players>, config: PlayerMuteConfig) -> Self {
        Self {
            db,
            players,
            config,
            offenses: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn handle_chat_msg(
        &self,
        bf4: Arc<Bf4Client>,
        player: Player,
        mut msg: AsciiString,
    ) -> RconResult<()> {

        msg.make_ascii_lowercase();
        let split = msg.as_str()
            .split(' ')
            .filter(|&s| !s.is_empty())
            .collect::<Vec<_>>();

        if split.is_empty() {
            return Ok(())
        }

        match split[0].chars().next().unwrap() {
            '/' | '!' | '@' | '#' => (),
            _ => return Ok(())
        }

        let cmd = split[0]
            .trim_start_matches('/')
            .trim_start_matches('!')
            .trim_start_matches('@')
            .trim_start_matches('#');

        if cmd.eq("mute") || cmd.eq("unmute") {
            // Admin check
            if !self.config.mute_permissions.contains_key(&player.name) {
                // TODO: Should we notify about no permissions or just ignore?
                return Ok(())
            }

            if cmd.eq("mute") && split.len() < 4 {
                let _ = bf4.say("\nMute player with: /mute <soldierName> <[r|d<days>|p]> <reason>\n\t/mute xfileFIN d2 reason\n\t/mute xfileFIN p permanent mute", &player).await;

                return Ok(())
            }
            else if cmd.eq("unmute") && split.len() < 2 {
                let _ = bf4.say("\nUnMute player with: /unmute <soldierName>", &player).await;

                return Ok(())
            }

            let target = split[1];
            info!("Target player {}", target);

            match self.players.get_best_player_match(target).await {
                Ok(best_match) => {
                    info!("Match for {} / {}", best_match.name, best_match.eaid);

                    match cmd {
                        "mute" => {
                            let reason = split[3..].join(" ");

                            let mut mute_type = split[2].chars();
                            let mut mute_player = BfoxMutedPlayer {
                                eaid: best_match.eaid.to_string(),
                                type_: 0,
                                end_date: None,
                                kicks: None,
                                reason: Some(reason.clone())
                            };

                            match mute_type.next().unwrap() {
                                'r' => {
                                    mute_player.type_ = MuteType::Round as i32;

                                },
                                'd' => {
                                    mute_player.type_ = MuteType::Days as i32;
                                    if split[2].len() < 2 {
                                        let _ = bf4.say("Invalid mute type\n\tr (round)\n\td (days) -> d2 (two days)\n\tp (permanent)", &player).await;
                                        return Ok(())
                                    }
                                    match mute_type.as_str().parse::<i64>() {
                                        Ok(n) => {
                                            todo!("There's some uncommented code here because of the diesel to sqlx migration"); // TODO: !!!
                                            // mute_player.end_date = Some(Utc::now().naive_utc().add(Duration::days(n)).date());
                                        },
                                        Err(_) => {
                                            let _ = bf4.say("Invalid mute type\n\tr (round)\n\td (days) -> d2 (two days)\n\tp (permanent)", &player).await;
                                            return Ok(())
                                        },
                                    }
                                },
                                'p' => {
                                    mute_player.type_ = MuteType::Permanent as i32;

                                },
                                _ => {
                                    let _ = bf4.say("Invalid mute type\n\tr (round)\n\td (days) -> d2 (two days)\n\tp (permanent)", &player).await;
                                    return Ok(())
                                }
                            }

                            let mut lock = self.offenses.lock().await;
                            if self.try_add_mute(best_match.eaid, mute_player, &mut lock) {
                                let _ = bf4.say(format!("{} has been muted for {}", best_match.name, reason), &player).await;
                                let _ = bf4.say(format!("You have been muted for {}", reason), &best_match).await;
                                return Ok(())
                            }
                        },
                        "unmute" => {
                            match self.try_get_muted_player(&best_match.eaid.to_string()) {
                                Some(_) => {
                                    let mut lock = self.offenses.lock().await;
                                    if self.try_remove_mute(&best_match.eaid, &mut lock) {
                                        let _ = bf4.say(format!("Mute for player {} has been removed.", best_match.name), &player).await;
                                        let _ = bf4.say("You have been unmuted.", &best_match).await;
                                        return Ok(())
                                    }
                                },
                                _ => {
                                    let _ = bf4.say(format!("Player {} wasn't muted.", best_match.name), &player).await;
                                    return Ok(())
                                }
                            }
                        },
                        _ => warn!("Something went wrong in the mute addon.")
                    }
                },
                Err(error) => {
                    match error {
                        MatchError::NoMatches => {
                            warn!("No matches for {}", target);

                            let _ = bf4.say(format!("Couldn't find player with name {}", target), player).await;
                        },
                        MatchError::TooMany => {
                            warn!("Too many matches for {}", target);

                            let _ = bf4.say(format!("Found too many players with name {}", target), player).await;
                        },
                    }
                },
            }
        }

        Ok(())
    }

    async fn update_players(&self) -> anyhow::Result<()> {
        debug!("Updating muted players");

        let muted_players = self.db.get_muted_players().await?;
        debug!("Muted players: {:#?}", muted_players);

        let mut map = self.offenses.lock().await;
        // Remove people who have gotten unmuted/mute has expired
        map.retain(|&key, _| muted_players.iter().any(|p| key.to_string() == p.eaid));

        // Add missing muted people
        for muted_player in muted_players.iter() {
            let eaid = Eaid::new(&AsciiString::from_ascii(muted_player.eaid.clone()).unwrap());
            if let Ok(eaid) = eaid {
                map.entry(eaid).or_insert(MutedPlayerInfo {
                    infractions: 0,
                    mute_type: muted_player.type_.try_into().unwrap(),
                    reason: muted_player.reason.clone()
                });

                debug!("Added or updated mute for: {:#?}", eaid);
            }
        }

        Ok(())
    }

    async fn remove_round_mutes(&self) {
        debug!("Removing round muted players");

        let lock = self.offenses.lock().await;
        let ids = lock.iter()
            .filter(|(_, mp)| mp.mute_type == MuteType::Round)
            .map(|(eaid, _)| eaid.to_string())
            .collect_vec();
        drop(lock);

        self.db.delete_muted_players(&ids);
    }

    async fn add_kicked(&self, eaid: &Eaid) {
        debug!("Adding kick for {}", eaid.to_string());

        let mut player = self.db.get_muted_player(eaid.to_string()).await?;
        player.kicks = Some(player.kicks.unwrap_or(0) + 1);
        match replace_into_muted_player(&con, &player) {
            Ok(_) => debug!("Added kick for {}", eaid.to_string()),
            Err(_) => debug!("Failed to add kick for {}", eaid.to_string()),
        }
    }

    fn try_get_muted_player(&self, eaid: &str) -> Option<BfoxMutedPlayer> {
        match establish_connection() {
            Ok(con) => {
                if let Ok(player) = get_muted_player(&con, eaid) {
                    return Some(player);
                }
            },
            Err(error) => error!("Failed to connect to database: {}", error),
        }

        None
    }

    fn try_remove_mute(&self, eaid: &Eaid, muted_players: &mut HashMap<Eaid, MutedPlayerInfo>) -> bool {
        debug!("Removing player {} from muted players", eaid.to_string());

        match establish_connection() {
            Ok(con) => {
                let result = delete_muted_player(&con, eaid.to_string());
                match result {
                    Ok(_) => {
                        debug!("Removed mute from: {:#?}", eaid);
                        if muted_players.remove(eaid).is_none() {
                            return false
                        }

                        return true
                    },
                    Err(err) => error!("Error trying to remove mute from {}: {}", eaid, err),
                }
            },
            Err(error) => error!("Failed to connect to database: {}", error),
        }

        false
    }

    fn try_add_mute(&self, eaid: Eaid, player: BfoxMutedPlayer, muted_players: &mut HashMap<Eaid, MutedPlayerInfo>) -> bool {
        debug!("Adding player {} to muted players", eaid.to_string());

        match establish_connection() {
            Ok(con) => {
                let result = replace_into_muted_player(&con, &player);
                match result {
                    Ok(_) => {
                        muted_players.entry(eaid).or_insert(MutedPlayerInfo {
                            infractions: 0,
                            mute_type: player.type_.try_into().unwrap(),
                            reason: player.reason.clone()
                        });

                        debug!("Added mute for: {:#?}", eaid);
                        return true
                    },
                    Err(err) => error!("Error trying to remove mute from {}: {}", eaid, err),
                }
            },
            Err(error) => error!("Failed to connect to database: {}", error),
        }

        false
    }
}