#![allow(clippy::new_without_default)]
#![feature(trait_upcasting)]

#[allow(unused_imports)] // we use maplin in tests
#[macro_use] extern crate maplit;
#[macro_use] extern crate log;
#[macro_use] extern crate multimap;
#[macro_use] extern crate derive_more;

use ascii::{IntoAsciiString};
use dotenv::dotenv;
use guard::Guard;
use itertools::Itertools;
use players::Players;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use battlefox_shared::rabbitmq::RabbitMq;
use weaponforcer::WeaponEnforcer;
use playerreport::PlayerReport;
use std::any::Any;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::time::Instant;
use std::{env::var, sync::Arc};
use vips::Vips;

use battlefield_rcon::{bf4::{Bf4Client}, rcon::{self, RconConnectionInfo}};
use mapmanager::{MapManager, PopState};
use mapvote::{
    config::{MapVoteConfig, MapVoteConfigJson},
    Mapvote,
};

use crate::admins::Admins;
use crate::{playermute::{PlayerMute, PlayerMuteConfig}, weaponforcer::WeaponEnforcerConfig};
use crate::playerreport::PlayerReportConfig;

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

fn get_rcon_coninfo() -> rcon::RconResult<RconConnectionInfo> {
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

#[derive(Debug)]
enum ConfigError {
    Serde(serde_yaml::Error),
    Io(std::io::Error),
}

fn load_config<T: DeserializeOwned>(path: &str) -> Result<T, ConfigError> {
    info!("Loading {}", path);
    let mut file = File::open(path).map_err(ConfigError::Io)?;
    let mut s = String::new();
    file.read_to_string(&mut s).map_err(ConfigError::Io)?;
    let t: T = serde_yaml::from_str(&s).map_err(ConfigError::Serde)?;
    Ok(t)
}

/// Convenience thing for loading stuff from Json.
#[derive(Debug, Serialize, Deserialize)]
struct MapManagerConfig {
    enabled: bool,
    pop_states: Vec<PopState>,

    vehicle_threshold: usize,
    leniency: usize,
}

#[async_trait::async_trait]
pub trait Plugin : Any + Send + Sync + 'static {
    /// "mapman" will result in for example `configs/mapman.yaml`.
    fn name() -> &'static str where Self: Sized;

    async fn run(self: Arc<Self>, bf4: Arc<Bf4Client>);
}

pub struct App {
    plugins: BTreeMap<String, Arc<dyn Plugin>>,
}

impl App {
    pub fn new() -> Self {
        Self {
            plugins: BTreeMap::new()
        }
    }

    fn has_plugin<P: Plugin, C: DeserializeOwned>(&mut self, f: impl FnOnce(C) -> P) -> Result<Arc<P>, ConfigError> {
        let config_path = format!("configs/{}.yaml", P::name());
        let config: C = load_config(&config_path)?;
        let p = Arc::new(f(config));

        let exists = self.plugins.insert(P::name().to_string(), p.clone());
        if exists.is_some() {
            panic!("Double-loading of plugins is forbidden. Plugin: {}", P::name());
        }

        Ok(p)
    }

    /// Invoke `run` on every loaded plugin, then wait for completion.
    pub async fn run(&mut self, bf4: Arc<Bf4Client>) {
        let jhs = self.plugins.values()
            .map(|p| {
                let bf4 = bf4.clone();
                let p = p.clone();
                tokio::spawn(p.run(bf4))
            })
            .collect_vec();

        // TODO: Abort everything when even just one jh returns prematurely and with Err.
        for jh in jhs {
            jh.await.unwrap()
        }
    }
}

#[allow(clippy::or_fun_call)]
#[tokio::main]
async fn main() -> rcon::RconResult<()> {
    dotenv().ok(); // load (additional) environment variables from `.env` file in working directory.
    logging::init_logging();
    info!("This is BattleFox {}", GIT_DESCRIBE);
    let _ = UPTIME.elapsed(); // get it, so that it initializes with `Instant::now()`.

    let mut app = App::new();
    let admins = app.has_plugin(Admins::new).unwrap();

    let coninfo = get_rcon_coninfo()?;
    info!("Connecting to {}:{} with password ***...", coninfo.ip, coninfo.port);
    let bf4 = Bf4Client::connect((coninfo.ip, coninfo.port), coninfo.password).await.unwrap();
    trace!("Connected!");

    // actually start all the plugins and wait for them to finish.
    app.run(bf4).await;

    // let mut rabbitmq = RabbitMq::new(None);
    // if let Err(why) = rabbitmq.run().await {
    //     error!("Error running rabbitmq publisher - Player Reports won't work: {:?}", why);
    // }

    // let players = Arc::new(Players::new());

    // let vips = Arc::new(Vips::new());

    // let weaponforcer_config : WeaponEnforcerConfig = load_config(&format!("{}weaponforcer.yaml", configs_path)).unwrap();
    // let weaponforcer = WeaponEnforcer::new(weaponforcer_config);

    // // let playerreport_config : PlayerReportConfig = load_config(&format!("{}playerreport.yaml", configs_path)).unwrap();
    // // let playerreport = PlayerReport::new(players.clone(), rabbitmq, playerreport_config);

    // let playermute_config : PlayerMuteConfig = load_config(&format!("{}playermute.yaml", configs_path)).unwrap();
    // let playermute = PlayerMute::new(players.clone(), playermute_config);

    // // let commands = Arc::new(Commands::new());

    // let mapman_config: MapManagerConfig = load_config(&format!("{}mapman.yaml", configs_path)).unwrap();
    // let mapman = Arc::new(MapManager::new(
    //     Guard::new(mapman_config.pop_states).expect("Failed to validate map manager config"),
    //     mapman_config.vehicle_threshold,
    //     mapman_config.leniency,
    //     mapman_config.enabled,
    // ));

    // let mapvote_config: MapVoteConfigJson = load_config(&format!("{}mapvote.yaml", configs_path)).unwrap();
    // let mapvote = Mapvote::new(
    //     mapman.clone(),
    //     vips.clone(),
    //     players.clone(),
    //     MapVoteConfig::from_json(mapvote_config),
    // )
    // .await;




    // { // Testing stuff
    //     let player = Player {
    //         name: AsciiString::from_ascii("xfileFIN").unwrap(),
    //         eaid: Eaid::new(&AsciiString::from_ascii("EA_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX").unwrap()).unwrap(),
    //     };
    //     // for i in 0..10 {
    //     //     bf4.say(format!("{}", i).repeat(20), Visibility::All).await.unwrap();
    //     //     sleep(Duration::from_millis(2000)).await;
    //     // }

    //     for map in battlefield_rcon::bf4::Map::all() {
    //         let mut msg = "\t".to_string();
    //         for minlen in 2..5 {
    //             msg += &format!("{}", map.tab4_prefixlen_wvehicles(minlen, false));
    //             // let upper = map.short()[..minlen].to_ascii_uppercase();
    //             // let lower = map.short()[minlen..].to_string();
    //             // msg += &format!("[\t{}{}\t]  ", upper, lower); // TODO: trim last \t of last chunk item
    //         }
    //         msg += "|";
    //         bf4.say(msg, &player).await.unwrap();
    //         sleep(Duration::from_millis(1500)).await;
    //     }

    //     // bf4.say("Test", Player {
    //     //     name: AsciiString::from_ascii("xfileFIN").unwrap(),
    //     //     eaid: Eaid::new(&AsciiString::from_ascii("EA_FCB11161E04E98494AEB5A91A9329486").unwrap()).unwrap(),
    //     // }).await.unwrap();

    //     return Ok(());
    // }

    // // start parts.
    // let mut jhs = Vec::new();

    // // let bf4clone = bf4.clone();
    // // jhs.push(tokio::spawn(async move { commands.run(bf4clone).await }));

    // let bf4clone = bf4.clone();
    // jhs.push(tokio::spawn(async move { players.run(bf4clone).await }));

    // let bf4clone = bf4.clone();
    // jhs.push(tokio::spawn(async move { mapvote.run(bf4clone).await }));

    // let bf4clone = bf4.clone();
    // jhs.push(tokio::spawn(async move { mapman.run(bf4clone).await }));

    // let bf4clone = bf4.clone();
    // jhs.push(tokio::spawn(async move { weaponforcer.run(&bf4clone).await }));

    // // let bf4clone = bf4.clone();
    // // jhs.push(tokio::spawn(async move { playerreport.run(bf4clone).await }));

    // let bf4clone = bf4.clone();
    // jhs.push(tokio::spawn(async move { playermute.run(bf4clone).await }));

    // // Wait for all our spawned tasks to finish.
    // // This'll happen at shutdown, or never, when you CTRL-C.
    // for jh in jhs.drain(..) {
    //     jh.await.unwrap()?
    // }

    trace!("Exiting gracefully :)");

    Ok(())
}
