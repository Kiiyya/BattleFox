use std::collections::BTreeMap;
use std::fmt::Display;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

use async_trait::async_trait;
use battlefield_rcon::bf4::{Bf4Client, Event, Player, Weapon, Visibility};
use battlefield_rcon::rcon::RconResult;
use lerp::Lerp;
use serde::{Deserialize, Serialize};

use crate::Plugin;
use crate::players::Players;

// // Serde doesn't allow literals as default values, apparently? Yikes.
// fn default_badness() -> f32 { 1.0 }
fn const_true() -> bool { true }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
	#[serde(default = "const_true")]
	enabled: bool,

	// #[serde(default = "default_badness")]
	// vehicle_badness: f32,
	// #[serde(default = "default_badness")]
	// explosive_badness: f32,
	// #[serde(default = "default_badness")]
	// gun_badness: f32,

	// badness_threshold_kill: f32,
	badness_threshold_kick: f32,

	/// Seconds -> badness. Interpolated linearly in between.
	badness_time_scale: BTreeMap<u64, f32>,

	/// Amount of minutes after which history entries are to be trimmed.
	trim_history_minutes: u64,
}

impl Config {
	fn interpolate_time_scale(&self, duration: Duration) -> f32 {
		// Get iterator where we're only interested in (-inf, duration].
		// For example with `duration = 2`, we may find a key of `1` seconds.
		let (&lower_secs, &lower_badness) = self.badness_time_scale
			.range(..= duration.as_secs())
			.last()
			.expect("TeamKilling badness_time_scale is assumed to have a value for zero seconds!");

		// Get the element directly after lower. For example if `lower_secs` is 1, then we
		// may find a key of `5` seconds.
		let upper = self.badness_time_scale
			.range(lower_secs + 1 ..)
			.next(); // first element with this property

		if let Some((&upper_secs, &upper_badness)) = upper {
			let span = (upper_secs - lower_secs) as f32;
			let duration_in_span = duration.as_secs_f32() - (lower_secs as f32);
			let t = duration_in_span / span;
			lower_badness.lerp_bounded(upper_badness, t) // linearly interpolate in between
		} else {
			// just clip
			lower_badness
		}
	}
}

#[derive(Debug, Clone)]
struct HistEntry {
	timestamp: Instant,
	weapon: Weapon,
	victim: Player,
}

impl HistEntry {
	fn badness(&self, config: &Config) -> f32 {
		config.interpolate_time_scale(self.timestamp.elapsed())
	}
}

impl Display for HistEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{} ago: Killed {} with {}",
			self.timestamp.elapsed().as_secs(),
			self.victim.name,
			self.weapon,
		)
    }
}

#[derive(Debug, Default, Clone)]
struct PlayerHistory {
	teamkills: Vec<HistEntry>,
}

impl Display for PlayerHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for entry in &self.teamkills {
			writeln!(f, "{}", entry)?;
		}
		Ok(())
    }
}

impl PlayerHistory {
	/// Sum of the badness of all history entries.
	fn badness(&self, config: &Config) -> f32 {
		self.teamkills.iter()
		 	.map(|hist| hist.badness(config))
			.sum()
	}

	fn trim(&mut self, minutes: u64) {
		let minutes = Duration::from_secs(60 * minutes);
		self.teamkills.retain(|entry| entry.timestamp.elapsed() < minutes);
	}
}

struct Inner {
	histories: BTreeMap<Player, PlayerHistory>,
}

pub struct TeamKilling {
	config: Config,
	players: Arc<Players>,
	inner: Mutex<Inner>,
}

impl TeamKilling {
	pub fn new(config: Config, players: Arc<Players>) -> Self {
		Self {
			config,
			players,
			inner: Mutex::new(Inner {
				histories: BTreeMap::new(),
			})
		}
	}

	async fn event(self: Arc<Self>, bf4: Arc<Bf4Client>, event: Event) -> RconResult<()> {
		match event {
			Event::Authenticated { player } => {
				let _ = bf4.admin_add(&player.name, 1).await;
			},
			Event::Leave { player , .. } => {
				let _ = bf4.admin_remove(&player).await;
			}
			Event::Kill {killer: Some(killer), victim, weapon, .. } => {
				if let Some(killer2) = self.players.player(&killer).await {
					if let Some(victim2) = self.players.player(&victim).await {
						if killer2.team == victim2.team {
							let hist = {
								let mut lock = self.inner.lock().unwrap();
								let hist = lock.histories.entry(killer).or_default();
								hist.teamkills.push(HistEntry {
									timestamp: Instant::now(),
									weapon,
									victim,
								});
								hist.trim(self.config.trim_history_minutes);
								hist.clone()
								// lock dropped here.
							};

							let badness = hist.badness(&self.config);
							if badness >= self.config.badness_threshold_kick {
								info!("Player {} achieved teamkilling badness {} with history:\n{}",
									killer2.player.name,
									badness,
									hist,
								);
								let _ = bf4.say(format!("Kicking {} for excessive teamkilling.", killer2.player.name), Visibility::All).await;
								let _ = bf4.kick(killer2.player.name, "Teamkilling").await;
							}
						}
					}
				}
			}
			Event::Chat { player, msg, .. } => {
				match msg.as_str() {
					"/tk badness" => {
						let x = {
							let lock = self.inner.lock().unwrap();
							lock.histories.get(&player).map(|hist| hist.badness(&self.config))
						};
						if let Some(badness) = x {
							let _ = bf4.say(format!("Your teamkilling badness: {}", badness), player).await;
						} else {
							let _ = bf4.say("No recent teamkilling history", player).await;
						}
					},
					"/tk hist" => {
						let x = {
							let lock = self.inner.lock().unwrap();
							lock.histories.get(&player).cloned()
						};
						if let Some(hist) = x {
							let mut lines = Vec::new();
							for entry in &hist.teamkills {
								lines.push(format!("{}", entry));
							}
							let _ = bf4.say_lines(lines, player).await;
							// let _ = bf4.say(format!("Your teamkilling badness: {}", badness), player).await;
						} else {
							let _ = bf4.say("No recent teamkilling history", player).await;
						}
					},
					_ => ()
				}
			}
			_ => ()
		}
		Ok(())
	}
}

#[async_trait]
impl Plugin for TeamKilling {
    const NAME: &'static str = "teamkilling";
	fn enabled(&self) -> bool { self.config.enabled }

	async fn start(self: &Arc<Self>, bf4: &Arc<Bf4Client>) {
		let self_clone = self.clone();
		tokio::spawn(async move {
			// every 10 minutes, trim teamkilling entries and yeet empty ones.
			loop {
				tokio::time::sleep(Duration::from_secs(60 * 10)).await;
				let mut lock = self_clone.inner.lock().unwrap();
				lock.histories.iter_mut()
					.for_each(|(_, hist)| hist.trim(self_clone.config.trim_history_minutes));
				lock.histories.retain(|_, hist| !hist.teamkills.is_empty());
			}
		});

		// put all players into admin list
		// let self_clone = self.clone();
		let bf4 = bf4.clone();
		tokio::spawn(async move {
			loop {
				// let players = self_clone.players.players(&*bf4).await;
				let players = bf4.list_players(Visibility::All).await.unwrap(); // bad unwrap... :(
				for player in players.iter() {
					let _ = bf4.admin_add(&player.player_name, 1);
				}
				// tokio::time::sleep(Duration::from_secs(60 * 20)).await;
			}
		});
	}

    async fn event(self: Arc<Self>, bf4: Arc<Bf4Client>, event: Event) -> RconResult<()> {
		self.event(bf4, event).await
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn interpolate() {
		let cfg = Config {
			enabled: true,
			// vehicle_badness: 0.0,
			// explosive_badness: 0.0,
			// gun_badness: 0.0,
			badness_threshold_kick: 0.0,
			trim_history_minutes: 10000,
			badness_time_scale: btreemap! {
				0 => 0.0,
				5 => 50.0,
				10 => 100.0,
			},
		};

		for x in 0..20 {
			println!("    x = {x} results in badness {}", cfg.interpolate_time_scale(Duration::from_secs(x)));
			println!();
		}
	}
}
