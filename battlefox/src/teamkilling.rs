use std::collections::{BTreeMap, BTreeSet};
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

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
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

impl HistEntry {
	fn badness(&self, config: &Config) -> f32 {
		config.interpolate_time_scale(self.timestamp.elapsed())
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

#[derive(Debug, Clone, Copy)]
enum DebugSatk {
	SuicidesAsTk,
	KillsAsTk,
}

struct Inner {
	histories: BTreeMap<Player, PlayerHistory>,
	debug_count_suicides_as_tk: BTreeMap<Player, DebugSatk>,
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
				debug_count_suicides_as_tk: BTreeMap::new(),
			})
		}
	}

	async fn event(self: Arc<Self>, bf4: Arc<Bf4Client>, event: Event) -> RconResult<()> {
		match event {
			Event::Authenticated { player } => {
				let _ = bf4.admin_add(&player.name, 1).await;
			},
			Event::Leave { player , .. } => {
				let _ = bf4.admin_remove(&player.name).await;
			}
			Event::Kill {killer: Some(killer), victim, weapon, .. } => {
				if let Some(killer2) = self.players.player(&killer).await {
					if let Some(victim2) = self.players.player(&victim).await {
						// Because of the debug stuff, we need to mutex-lock every kill event unfortunately.
						// Without the debug stuff, this code would be a lot simpler, as you would instead
						// check whether killer.team == victim.team, and *then* lock the mutex.

						// We consider this kill a teamkill exactly when teamkill_history is Some(_).
						let teamkill_history = {
							let mut lock = self.inner.lock().unwrap();
							let debug = match lock.debug_count_suicides_as_tk.get(&killer) {
								Some(&DebugSatk::KillsAsTk) => true, // all kills are considered teamkills.
								Some(&DebugSatk::SuicidesAsTk) => killer == victim, // only suicides
								None => false, // Normal, non-debug behaviour.
							};

							// This value determines whether the kill will be consider a tk.
							let is_tk = killer2.team == victim2.team || debug;

							// If we do consider the kill to be a tk, then append it to the history,
							// and return that history as Some(hist).
							is_tk.then(|| {
								let hist = lock.histories.entry(killer).or_default();
								hist.teamkills.push(HistEntry {
									timestamp: Instant::now(),
									weapon: weapon.clone(),
									victim,
								});
								hist.trim(self.config.trim_history_minutes);
								hist.clone()
							})
						};

						if let Some(hist) = teamkill_history {
							let badness = hist.badness(&self.config);
							trace!("Player {} teamkilled {} with {weapon}. Badness is now at {badness}", killer2.player.name, victim2.player.name);
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
					"/tk debug" | "/tk debug off" | "/tk debug suicides" | "/tk debug kills" => {
						let mode = {
							let mut lock = self.inner.lock().unwrap();
							match msg.as_str() {
								"/tk debug off" => {
									lock.debug_count_suicides_as_tk.remove(&player);
									None
								},
								"/tk debug" | "/tk debug suicides" => {
									lock.debug_count_suicides_as_tk.insert(player.clone(), DebugSatk::SuicidesAsTk);
									Some(DebugSatk::SuicidesAsTk)
								},
								"/tk debug kills" => {
									lock.debug_count_suicides_as_tk.insert(player.clone(), DebugSatk::KillsAsTk);
									Some(DebugSatk::KillsAsTk)
								},
								_ => unreachable!(),
							}
						};

						let _ = bf4.say(format!("Tk debug is now {mode:?}"), player).await;
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
				let players: BTreeSet<_> = bf4.list_players(Visibility::All).await.unwrap() // bad unwrap... :(
					.iter().map(|p| p.player_name.clone())
					.collect();
				let admins: BTreeSet<_> = bf4.admin_list().await.unwrap() // bad unwrap... :(
					.iter().map(|(adm, _)| adm.clone())
					.collect();

				// visit admins who are not on the server, and remove them.
				for admin in admins.difference(&players) {
					debug!("RCON-Admin {admin} was admin but current not in server. Removing.");
					bf4.admin_remove(admin).await.unwrap();
				}

				// visit players who are not admins, and add them to the admin list.
				for player in players.difference(&admins) {
					debug!("Player {player} is in server but not RCON-Admin. Adding with level 1.");
					if let Err(e) = bf4.admin_add(player, 1).await {
						warn!("Failed to add player {player} to RCON gameAdmin list: {e:?}");
					}
				}

				tokio::time::sleep(Duration::from_secs(60 * 20)).await;
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
	#[ignore]
	fn interpolate() {
		let cfg = Config {
			badness_time_scale: btreemap! {
				0 => 0.0,
				5 => 50.0,
				10 => 100.0,
			},
			..Default::default()
		};

		for x in 0..20 {
			println!("    x = {x} results in badness {}", cfg.interpolate_time_scale(Duration::from_secs(x)));
			println!();
		}
	}
}
