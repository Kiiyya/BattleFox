use std::{collections::{HashMap, HashSet}, time::Duration};

use ascii::{AsciiString, IntoAsciiString};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct MapVoteConfig {
    pub n_options: usize,

    /// Not even VIPs will be able to do more than this
    pub max_options: usize,

    pub max_noms_per_vip: usize,

    pub spammer_interval: Duration,

    pub endscreen_votetime: Duration,
    pub endscreen_post_votetime: Duration,

    pub vip_nom: String,
    pub vip_ad: String,

    pub animate: bool,
    pub animate_override: HashMap<AsciiString, bool>,

    pub options_minlen: usize,
    pub options_reserved: HashSet<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MapVoteConfigJson {
    pub n_options: usize,

    /// Not even VIPs will be able to do more than this
    pub max_options: usize,

    pub max_noms_per_vip: usize,

    pub spammer_interval: u64,

    pub endscreen_votetime: u64,
    pub endscreen_post_votetime: u64,

    pub vip_ad: String,
    pub vip_nom: String,

    pub animate: bool,
    pub animate_override: HashMap<String, bool>,

    pub options_minlen: usize,
    pub options_reserved: HashSet<String>,
}

impl MapVoteConfig {
    pub fn from_json(other: MapVoteConfigJson) -> Self {
        Self {
            n_options: other.n_options,
            max_options: other.max_options,
            max_noms_per_vip: other.max_noms_per_vip,
            spammer_interval: Duration::from_secs(other.spammer_interval),
            endscreen_votetime: Duration::from_secs(other.endscreen_votetime),
            endscreen_post_votetime: Duration::from_secs(other.endscreen_post_votetime),
            vip_nom: other.vip_ad,
            vip_ad: other.vip_nom,
            animate: other.animate,
            animate_override: other.animate_override.iter().map(|(k, v)| (k.clone().into_ascii_string().unwrap(), *v)).collect(),
            options_minlen: other.options_minlen,
            options_reserved: other.options_reserved,
        }
    }
}
