#![allow(clippy::new_without_default)]

use ascii::IntoAsciiString;
use dotenv::dotenv;
use guard::Guard;
use players::Players;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{env::var, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use vips::Vips;

use battlefield_rcon::{
    bf4::Bf4Client,
    rcon::{self, RconConnectionInfo},
};
use mapmanager::{pool::Vehicles, MapManager, PopState};
use mapvote::{Mapvote, config::MapVoteConfig};

pub mod guard;
pub mod mapmanager;
pub mod mapvote;
pub mod vips;
// pub mod minicache;
pub mod players;
mod stv;

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
    Serde(serde_json::Error),
    Io(std::io::Error),
}

async fn load_config<T: DeserializeOwned>(path: &str) -> Result<T, ConfigError> {
    println!("Loading {}", path);
    let mut file = tokio::fs::File::open(path).await.map_err(ConfigError::Io)?;
    let mut s = String::new();
    file.read_to_string(&mut s).await.map_err(ConfigError::Io)?;
    let t: T = serde_json::from_str(&s).map_err(ConfigError::Serde)?;
    Ok(t)
}

#[allow(dead_code)]
async fn save_config<T: Serialize>(path: &str, obj: &T) -> Result<(), ConfigError> {
    let mut file = tokio::fs::File::create(path)
        .await
        .map_err(ConfigError::Io)?;
    let s = serde_json::to_string_pretty(obj).map_err(ConfigError::Serde)?;
    file.write_all(s.as_bytes())
        .await
        .map_err(ConfigError::Io)?;

    Ok(())
}

/// Convenience thing for loading stuff from Json.
#[derive(Debug, Serialize, Deserialize)]
struct MapManagerConfig {
    pop_states: Vec<PopState<Vehicles>>,

    vehicle_threshold: usize,
    leniency: usize,
}

#[allow(clippy::or_fun_call)]
#[tokio::main]
async fn main() -> rcon::RconResult<()> {
    dotenv().ok(); // load (additional) environment variables from `.env` file in working directory.
    let coninfo = get_rcon_coninfo()?;

    let players = Arc::new(Players::new());

    let vips = Arc::new(Vips::new());

    // set up parts
    let mapman_config: MapManagerConfig = load_config("configs/mapman.json").await.unwrap();
    let mapman = Arc::new(MapManager::new(
        Guard::new(mapman_config.pop_states).expect("Failed to validate map manager config"),
        mapman_config.vehicle_threshold,
        mapman_config.leniency,
    ));
    let mapvote = Mapvote::new(mapman.clone(), vips.clone(), players.clone(), MapVoteConfig { n_options: 4 }).await;

    // connect
    println!(
        "Connecting to {}:{} with password ***...",
        coninfo.ip, coninfo.port
    );
    let bf4 = Bf4Client::connect(&coninfo).await.unwrap();
    println!("Connected!");

    // start parts.
    let mut jhs = Vec::new();

    let bf4clone = bf4.clone();
    jhs.push(tokio::spawn(async move { players.run(bf4clone).await }));

    let bf4clone = bf4.clone();
    jhs.push(tokio::spawn(async move { mapvote.run(bf4clone).await }));

    let bf4clone = bf4.clone();
    jhs.push(tokio::spawn(async move { mapman.run(bf4clone).await }));

    // Wait for all our spawned tasks to finish.
    // This'll happen at shutdown, or never, when you CTRL-C.
    for jh in jhs.drain(..) {
        jh.await.unwrap()?
    }

    Ok(())
}
