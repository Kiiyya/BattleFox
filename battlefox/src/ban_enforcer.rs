use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use battlefield_rcon::bf4::ban_list::{Ban, BanTimeout};
use battlefield_rcon::bf4::{Bf4Client, Event, BanListError, PlayerKickError};
use battlefield_rcon::rcon::RconResult;
use battlefox_database::{BfoxContext, DateTime};
use battlefox_database::adkats::bans::BanStatus;
use serde::{Deserialize, Serialize};

use crate::Plugin;
use crate::players::Players;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    enabled: bool,
}

pub struct BanEnforcer {
    config: Config,
    db: BfoxContext,
}

impl BanEnforcer {
    pub fn new(config: Config, _players: Arc<Players>, db: BfoxContext) -> Self {
        Self { config, db }
    }

    async fn event(self: Arc<Self>, bf4: Arc<Bf4Client>, event: Event) -> RconResult<()> {
        #[allow(clippy::single_match)]
        match event {
            Event::Authenticated { player } => {
                match self.db.get_ban(format!("{}", player.eaid)).await {
                    Ok(Some(ban)) => {
                        let banned = if ban.status == BanStatus::Active {
                            let now = DateTime::now_utc();
                            let is_banned_time = now < ban.end;

                            if !is_banned_time {
                                warn!("Ban for player {player} is \"Active\" but endTime is in the past. All times (assumed) in UTC. Now = {}, ban_end = {}", &now, &ban.end);
                            }
                            is_banned_time
                        } else {
                            false
                        };

                        if banned {
                            info!("Player {player} is banned, and will be kicked via tempban for one second: {ban:#?}");

                            match bf4.ban_add(
                                Ban::Guid(player.eaid),
                                BanTimeout::Time(Duration::from_secs(1)), // I guess rcon will remove this by itself?
                                Some(ban.reason.clone()) // reason
                            ).await {
                                Ok(()) => (),
                                Err(BanListError::BanListFull) => warn!("Ban list is full?!"),
                                Err(BanListError::NotFound) => unreachable!(),
                                Err(BanListError::Rcon(rcon_err)) => error!("Failed to tempban player for a second: {rcon_err:?}"),
                            }

                            match bf4.kick(player.name, ban.reason).await {
                                Ok(()) => (),
                                Err(PlayerKickError::PlayerNotFound) => (),
                                Err(PlayerKickError::Rcon(rcon_err)) => error!("Failed to kick player: {rcon_err:?}"),
                            }
                        } else {
                            debug!("Player {player} is in adkats_bans, but the ban has expired.");
                        }
                    },
                    Ok(None) => (), // player is not banned
                    Err(e) => error!("While checking player {player} for ban, got db error, but will ignore it and continue: {e}"),
                }
            },
            _ => (),
        }

        Ok(())
    }
}

#[async_trait]
impl Plugin for BanEnforcer {
    const NAME: &'static str = "ban_enforcer";
    fn enabled(&self) -> bool { self.config.enabled }

    async fn event(self: Arc<Self>, bf4: Arc<Bf4Client>, event: Event) -> RconResult<()> {
        self.event(bf4, event).await
    }
}
