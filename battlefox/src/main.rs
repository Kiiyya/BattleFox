#![allow(clippy::new_without_default)]

#[allow(unused_imports)] // we use maplin in tests
#[macro_use] extern crate maplit;
#[macro_use] extern crate log;
#[macro_use] extern crate multimap;
#[macro_use] extern crate derive_more;

use ascii::{IntoAsciiString};
use async_trait::async_trait;
use battlefield_rcon::bf4::Event;
use battlefield_rcon::rcon::RconResult;
use dotenv::dotenv;
use futures::StreamExt;
use itertools::Itertools;
use players::Players;
use serde::{de::DeserializeOwned};
use thiserror::Error;
use weaponforcer::WeaponEnforcer;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Instant;
use std::{env::var, sync::Arc};
use vips::Vips;

use battlefield_rcon::{bf4::Bf4Client, rcon::RconConnectionInfo};
use mapmanager::MapManager;
use mapvote::{
    config::{MapVoteConfig, MapVoteConfigJson},
    Mapvote,
};

use crate::admins::Admins;
use crate::playermute::PlayerMute;

pub mod guard;
// pub mod commands;
pub mod mapmanager;
pub mod mapvote;
pub mod vips;
pub mod admins;
pub mod players;
mod stv;
pub mod weaponforcer;
pub mod playerreport;
pub mod humanlang;
mod logging;
mod playermute;

// Instead of `cargo build`, set env vars:
//     RUSTFLAGS='--cfg take_git_version_from_env'
//     GIT_DESCRIBE='823we8fgse8f7gasef7238r27wef'
// And then `cargo build`.
#[cfg(not(take_git_version_from_env))]
const GIT_DESCRIBE : &str = git_version::git_version!(); // if Rust-Analyzer complains, disable the `unresolved-macro-call` diagnostic: https://github.com/rust-analyzer/rust-analyzer/issues/8477#issuecomment-817736916
#[cfg(take_git_version_from_env)]
const GIT_DESCRIBE : &str = env!("GIT_DESCRIBE");

lazy_static::lazy_static! {
    static ref UPTIME: Instant = Instant::now();
}

fn get_rcon_coninfo() -> anyhow::Result<RconConnectionInfo> {
    let ip = var("BFOX_RCON_IP").unwrap_or_else(|_| "127.0.0.1".into());
    let port = var("BFOX_RCON_PORT")
        .unwrap_or_else(|_| "47200".into())
        .parse::<u16>()
        .unwrap();
    let password = var("BFOX_RCON_PASSWORD").unwrap_or_else(|_| "smurf".into());
    Ok(RconConnectionInfo {
        ip,
        port,
        password: password.into_ascii_string()?,
    })
}

#[derive(Error, Debug)]
enum ConfigError {
    #[error("Failed to deserialize config.")]
    Serde(#[from] serde_yaml::Error),
    #[error("Failed to open config file")]
    Io(#[from] std::io::Error),
}

fn load_config<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, ConfigError> {
    info!("Loading {}", path.as_ref().to_string_lossy());
    let mut file = File::open(path)?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;
    let t: T = serde_yaml::from_str(&s)?;
    Ok(t)
}

#[async_trait]
pub trait Plugin : Send + Sync + 'static {
    const NAME: &'static str;
    fn enabled(&self) -> bool { true }

    async fn start(self: &Arc<Self>, _bf4: &Arc<Bf4Client>) { }

    /// You *can* implement this, but you may be more interested in `event`.
    ///
    /// In case `run` is overridden, `event` does nothing.
    async fn run(self: Arc<Self>, bf4: Arc<Bf4Client>) -> RconResult<()> {
        if self.enabled() {
            self.start(&bf4).await;

            let mut stream = bf4.event_stream().await?;
            while let Some(event) = stream.next().await {
                match event {
                    Ok(event) => {
                        let bf4 = bf4.clone();
                        let self_clone = self.clone();
                        tokio::spawn(async move { self_clone.event(bf4, event).await });
                    },
                    Err(err) => {
                        error!("Plugin \"{}\" encountered bf4 error and has quit: {:?}", Self::NAME, err);
                        break;
                    },
                }
            }
        } else {
            info!("Plugin {} is disabled.", Self::NAME);
        }
        Ok(())
    }

    async fn event(self: Arc<Self>, _bf4: Arc<Bf4Client>, _ev: Event) -> RconResult<()> {
        // do nothing unless overridden.
        Ok(())
    }
}

/// Just a helper trait to avoid trait object and associated constants clashing.
#[async_trait]
trait Plugin2: Sync + Send {
    async fn run(self: Arc<Self>, bf4: Arc<Bf4Client>) -> RconResult<()>;
    fn name(&self) -> &'static str;
}

#[async_trait]
impl<T: Plugin> Plugin2 for T {
    async fn run(self: Arc<Self>, bf4: Arc<Bf4Client>) -> RconResult<()> {
        self.run(bf4).await
        // todo!()
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }
}

pub struct App {
    plugins: BTreeMap<String, Arc<dyn Plugin2>>,
}

impl App {
    pub fn new() -> Self {
        Self {
            plugins: BTreeMap::new()
        }
    }

    fn has_plugin<P: Plugin, C: DeserializeOwned>(&mut self, f: impl FnOnce(C) -> P) -> Result<Arc<P>, ConfigError> {
        let config: C = load_config(&format!("configs/{}.yaml", P::NAME))?;
        self.has_plugin_noconfig(f(config))
    }

    fn has_plugin_arc<P: Plugin, C: DeserializeOwned>(&mut self, f: impl FnOnce(C) -> Arc<P>) -> Result<Arc<P>, ConfigError> {
        let config: C = load_config(&format!("configs/{}.yaml", P::NAME))?;
        let p = f(config);
        let exists = self.plugins.insert(P::NAME.to_string(), p.clone());
        if exists.is_some() {
            panic!("Double-loading of plugins is forbidden. Plugin: {}", P::NAME);
        }
        Ok(p)
    }

    fn has_plugin_noconfig<P: Plugin>(&mut self, p: P) -> Result<Arc<P>, ConfigError> {
        let p = Arc::new(p);

        let exists = self.plugins.insert(P::NAME.to_string(), p.clone());
        if exists.is_some() {
            panic!("Double-loading of plugins is forbidden. Plugin: {}", P::NAME);
        }

        Ok(p)
    }

    /// Invoke `run` on every loaded plugin, then wait for completion.
    pub async fn run(&mut self, bf4: Arc<Bf4Client>) {
        let jhs = self.plugins.iter()
            .map(|(name, p)| {
                let bf4 = bf4.clone();
                let p = p.clone();
                let jh = tokio::spawn(async move { p.run(bf4).await });
                (name, jh)
            })
            .collect_vec();

        // TODO: Abort everything when even just one jh returns prematurely and with Err.
        for (name, jh) in jhs {
            match jh.await.unwrap() {
                Ok(_) => (),
                Err(err) => error!("Plugin {} has quit with error: {:?}", name, err),
            }
        }
    }
}

#[allow(clippy::or_fun_call)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok(); // load (additional) environment variables from `.env` file in working directory.
    logging::init_logging();
    info!("This is BattleFox {}", GIT_DESCRIBE);
    let _ = UPTIME.elapsed(); // get it, so that it initializes with `Instant::now()`.

    // Initialize plugins and their dependencies.
    let mut app = App::new();
    let _admins = app.has_plugin(Admins::new)?;
    let players = app.has_plugin_noconfig(Players::new())?;
    let vips = app.has_plugin_noconfig(Vips::new())?;
    let _weaponforcer = app.has_plugin(WeaponEnforcer::new)?;
    // let _playerreport = app.has_plugin(|c| PlayerReport::new(players.clone(), rabbitmq, c))?;
    let _playermute = app.has_plugin(|c| PlayerMute::new(players.clone(), c))?;
    let mapman = app.has_plugin(MapManager::new)?;
    let _mapvote = app.has_plugin_arc(|c: MapVoteConfigJson|
        Mapvote::new(mapman, vips, players, MapVoteConfig::from_json(c))
    )?;

    // Connect to RCON.
    let coninfo = get_rcon_coninfo()?;
    info!("Connecting to {}:{} with password ***...", coninfo.ip, coninfo.port);
    let bf4 = Bf4Client::connect((coninfo.ip, coninfo.port), coninfo.password).await.unwrap();
    trace!("Connected!");

    // Actually start all the plugins and wait for them to finish.
    app.run(bf4).await;

    Ok(())
}
