use std::time::Duration;

use serde::{Serialize, Deserialize};

#[derive(Debug)]
pub struct MapVoteConfig {
    pub n_options: usize,

    /// Not even VIPs will be able to do more than this
    pub max_options: usize,

    pub max_noms_per_vip: usize,

    pub spammer_interval: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MapVoteConfigJson {
    pub n_options: usize,

    /// Not even VIPs will be able to do more than this
    pub max_options: usize,

    pub max_noms_per_vip: usize,

    pub spammer_interval: u64,
}

impl MapVoteConfig {
    pub fn from_json(other: MapVoteConfigJson) -> Self {
        Self {
            n_options: other.n_options,
            max_options: other.max_options,
            max_noms_per_vip: other.max_noms_per_vip,
            spammer_interval: Duration::from_secs(other.spammer_interval),
        }
    }
}
