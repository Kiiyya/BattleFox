use std::{collections::{HashMap}, sync::Arc};

use ascii::AsciiString;
use battlefield_rcon::{bf4::{Bf4Client, Event, Player, Visibility}, rcon::RconResult};
use futures::StreamExt;
use serde::{Serialize, Deserialize};
use shared::{rabbitmq::RabbitMq, report::ReportModel};

use crate::players::{MatchError, Players};

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerReportConfig {
    enabled: bool,
    server_guid: Option<String>,
    bfacp_url: Option<String>,
    command: Option<String>
}

pub struct PlayerReport {
    players: Arc<Players>,
    rabbit: RabbitMq,
    config: PlayerReportConfig
}

impl PlayerReport {
    pub fn new(players: Arc<Players>, rabbit: RabbitMq, config: PlayerReportConfig) -> Arc<Self> {
        Arc::new(Self {
            players,
            rabbit,
            config
        })
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
        _vis: Visibility,
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

        match split[0].chars().next().unwrap() {
            '/' | '!' | '@' | '#' => (),
            _ => return Ok(())
        }

        let cmd = split[0]
            .trim_start_matches('/')
            .trim_start_matches('!')
            .trim_start_matches('@')
            .trim_start_matches('#');

        let command = self.config.command.as_ref().unwrap_or(&"report".to_string()).to_lowercase();

        if cmd.eq(&command) {
            if split.len() < 3 {
                let _ = bf4.say(format!("Report player with: /{0} soldiername reason", command), player).await;

                return Ok(())
            }

            let target = split[1];
            let reason = split[2..].join(" ");

            info!("Reported player {}", target);
            match self.players.get_best_player_match(target).await {
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
                        server_name,
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
        }

        Ok(())
    }
}