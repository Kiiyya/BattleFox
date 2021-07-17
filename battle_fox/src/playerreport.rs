use std::{collections::{BTreeMap, HashMap}, sync::Arc};

use ascii::AsciiString;
use battlefield_rcon::{bf4::{Bf4Client, Event, Player, Visibility}, rcon::RconResult};
use futures::StreamExt;
use serde::{Serialize, Deserialize};
use strsim::levenshtein;
use shared::{rabbitmq::RabbitMq, report::ReportModel};

use crate::players::{PlayerInServer, Players};

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerReportConfig {
    enabled: bool,
    server_guid: Option<String>,
    bfacp_url: Option<String>
}

pub struct PlayerReport {
    players: Arc<Players>,
    rabbit: RabbitMq,
    config: PlayerReportConfig
}

enum MatchError {
    NoMatches,
    TooMany,
}

impl PlayerReport {
    pub fn new(players: Arc<Players>, rabbit: RabbitMq, config: PlayerReportConfig) -> Arc<Self> {
        let myself = Arc::new(Self {
            players,
            rabbit,
            config
        });

        myself
    }

    pub async fn run(self: Arc<Self>, bf4: Arc<Bf4Client>) -> RconResult<()> {
        if !self.config.enabled {
            debug!("Player reporting is disabled");
            return Ok(());
        }

        // TODO: Count the times someone was reported. If reported, could report to Discord that the reported player left
        let mut reports : HashMap<Player, usize> = HashMap::new();

        let mut stream = bf4.event_stream().await?;
        while let Some(event) = stream.next().await {
            match event {
                Ok(Event::Chat { vis, player, msg }) => {
                    let bf4 = bf4.clone();
                    let playerreport = self.clone();

                    tokio::spawn(async move {
                        let _ = playerreport.handle_chat_msg(bf4, vis, player, msg).await;
                    });

                    // if msg.as_str().starts_with("/haha next map") && player.name == "PocketWolfy" {
                    //     let mapman = self.mapman.clone();
                    //     tokio::spawn(async move {
                    //         mapvote.handle_round_over(&bf4).await;
                    //     });
                    // } else {
                    //     tokio::spawn(async move {
                    //         let _ = mapvote.handle_chat_msg(bf4, vis, player, msg).await;
                    //     });
                    // }
                },
                Ok(Event::RoundOver { winning_team: _ }) => {
                    reports.clear();
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_chat_msg(
        self: Arc<Self>,
        bf4: Arc<Bf4Client>,
        vis: Visibility,
        player: Player,
        mut msg: AsciiString
    ) -> RconResult<()> {

        //println!("{} said: {}", player.name, msg);

        msg.make_ascii_lowercase();
        let split = msg.as_str()
            .split(' ')
            .filter(|&s| !s.is_empty())
            .collect::<Vec<_>>();

        if split.is_empty() {
            return Ok(())
        }

        match split[0] {
            "!report" | "/report" | "@report" | "#report" => {
                if split.len() < 3 {
                    let _ = bf4.say("Report player with: /report soldiername reason", player).await;

                    return Ok(())
                }

                let target = split[1];
                let reason = split[2..].join(" ");

                info!("Reported player {}", target);

                let playerreport = self.clone();
                let mut players = self.players.players(&bf4).await;

                match playerreport.get_best_player_match(&mut players, target).await {
                    Ok(Player { name, eaid }) => {
                        info!("Match for {} / {}", name, eaid);

                        let server_name = match bf4.server_info().await {
                            Ok(info) => info.server_name.to_string(),
                            Err(_) => "Not found".to_string(),
                        };
                        info!("ServerName {:?}", server_name);

                        let report = ReportModel { 
                            reporter: player.name.to_string(),
                            reported: name.to_string(),
                            reason: reason.to_string(),
                            server_name: server_name,
                            server_guid: self.config.server_guid.clone(),
                            bfacp_link: self.config.bfacp_url.clone()
                        };

                        match self.rabbit.queue_report(report).await {
                            Ok(_) => {
                                let _ = bf4.say(format!("Reported player {} for {}", name, reason), player).await;
                            },
                            Err(error) => {
                                warn!("Error queueing a report: {}", error);

                                let _ = bf4.say(format!("Error reporting player {} for {}", name, reason), player).await;
                            },
                        }
                    },
                    Err(error) => {
                        match error {
                            MatchError::NoMatches => {
                                warn!("No matches for {}", target);

                                let _ = bf4.say(format!("Reporting failed, couldn't find player with name {}", target), player).await;
                            },
                            MatchError::TooMany => {
                                warn!("Too many matches for {}", target);

                                let _ = bf4.say(format!("Reporting failed, found too many players with name {}", target), player).await;
                            },
                        }
                    },
                }
            },
            _ => { }
        }

        Ok(())
    }

    async fn get_best_player_match(self: &Self, players: &mut HashMap<Player, PlayerInServer>, target: &str) -> Result<Player, MatchError> {
        self.players_contains(players, target); // Remove to allow errors in typing, for example 'I' as 'l'

        let players_start_with = self.players_starts_with(players, target);

        if players_start_with.len() > 0 {
            let matches = self.get_levenshtein(&players_start_with, target);
            // for (key, value) in matches.iter() {
            //     println!("distance, player: {} {:?}", key, value);
            // }

            // let mut sorted: Vec<_> = matches.iter().collect();
            // println!("Not ordered {:?}", sorted);

            // sorted.sort_by_key(|a| a.0);
            // println!("Ordered{:?}", sorted);

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
            let matches = self.get_levenshtein(&players, target);
            // for (key, value) in matches.iter() {
            //     println!("distance, player: {} {:?}", key, value);
            // }

            // let mut sorted: Vec<_> = matches.iter().collect();
            // println!("Not ordered {:?}", sorted);

            // sorted.sort_by_key(|a| a.0);
            // println!("Ordered{:?}", sorted);

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

    fn players_contains(self: &Self, map: &mut HashMap<Player, PlayerInServer>, target: &str) {
        let target_lowercase = target.to_ascii_lowercase();
        map.retain(|key, value| {
            //println!("{} / {:?}", key, value);
    
            key.name.to_ascii_lowercase().to_string().contains(&target_lowercase)
        })
    }

    fn players_starts_with(self: &Self, map: &HashMap<Player, PlayerInServer>, target: &str) -> HashMap<Player, PlayerInServer> {
        let target_lowercase = target.to_ascii_lowercase();
        map.into_iter().filter_map(|(key, value)| {
            if key.name.to_ascii_lowercase().to_string().starts_with(&target_lowercase) {
                Some((key.to_owned(), value.to_owned()))
            } else {
                None
            }
        }).collect()
    }

    fn get_levenshtein(self: &Self, map: &HashMap<Player, PlayerInServer>, target: &str) -> BTreeMap<usize, Player> {
        let target_lowercase = target.to_ascii_lowercase();
        map.into_iter().filter_map(|(key, _value)| {
            Some((levenshtein(&target_lowercase, &key.name.to_ascii_lowercase().to_string()), key.to_owned()))
        }).collect()
    }
}