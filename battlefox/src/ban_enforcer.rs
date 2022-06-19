use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use battlefield_rcon::bf4::ban_list::{Ban, BanTimeout};
use battlefield_rcon::bf4::{Bf4Client, Event, BanListError, PlayerKickError};
use battlefield_rcon::rcon::RconResult;
use battlefox_database::better::BfoxDb;
use battlefox_database::entities::sea_orm_active_enums::BanStatus;
use chrono::{Utc, DateTime, TimeZone};
use serde::{Deserialize, Serialize};

use crate::Plugin;
use crate::players::Players;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
	enabled: bool,
}

pub struct BanEnforcer {
	config: Config,
	db: BfoxDb,
}

impl BanEnforcer {
	pub fn new(config: Config, _players: Arc<Players>, db: BfoxDb) -> Self {
		Self { config, db }
	}

	async fn event(self: Arc<Self>, bf4: Arc<Bf4Client>, event: Event) -> RconResult<()> {
		#[allow(clippy::single_match)]
		match event {
			Event::Authenticated { player } => {
				match self.db.get_ban(format!("{}", player.eaid)).await {
					Ok(Some((_, ban))) => {
						let now = Utc::now();
						let ban_end : DateTime<Utc> = Utc.from_utc_datetime(&ban.ban_end_time); // assume our data is UTC.

						let is_banned_time = now < ban_end;
						let is_banned_status = ban.ban_status == BanStatus::Active;

						if is_banned_time != is_banned_status {
							warn!("Ban end time and ban_status mismatch for player {player}. All times (assumed) in UTC. Now = {}, ban_end = {}, ban_status = {:?}", &now, &ban_end, ban.ban_status);
						}

						// ban expiry time is more important than the ban_status column.
						if is_banned_time {
							info!("Player {player} is banned, and will be kicked via tempban for two minutes: {ban:#?}");

							match bf4.ban_add(
								Ban::Guid(player.eaid),
								BanTimeout::Time(Duration::from_secs(120)), // I guess rcon will remove this by itself?
								Some("") // reason
							).await {
								Ok(()) => (),
								Err(BanListError::BanListFull) => warn!("Ban list is full?!"),
								Err(BanListError::NotFound) => unreachable!(),
								Err(BanListError::Rcon(rcon_err)) => error!("Failed to tempban player for two minutes: {rcon_err:?}"),
							}

							match bf4.kick(player.name, "").await {
								Ok(()) => (),
								Err(PlayerKickError::PlayerNotFound) => (),
								Err(PlayerKickError::Rcon(rcon_err)) => error!("Failed to kick player: {rcon_err:?}"),
							}
						} else {
							debug!("Player {player} is in , but the ban has expired.");
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
