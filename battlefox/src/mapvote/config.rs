use std::{collections::{HashMap, HashSet}, time::Duration};

use ascii::{AsciiString, IntoAsciiString};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct MapVoteConfig {
    pub enabled: bool,
    pub n_options: usize,

    /// Not even VIPs will be able to do more than this
    pub max_options: usize,

    pub max_noms_per_vip: usize,

    pub vote_start_interval: Duration,

    pub spammer_interval: Duration,

    pub endscreen_votetime: Duration,
    pub endscreen_post_votetime: Duration,

    pub vip_nom: String,
    pub vip_ad: String,

    pub announce_nominator: Option<bool>,
    pub vip_vote_weight: Option<usize>,

    pub animate: bool,
    pub animate_override: HashMap<AsciiString, bool>,

    pub options_minlen: usize,
    pub options_reserved_hidden: HashSet<String>,
    pub options_reserved_trie: HashSet<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MapVoteConfigJson {
    pub enabled: bool,
    pub n_options: usize,

    /// Not even VIPs will be able to do more than this
    pub max_options: usize,

    pub max_noms_per_vip: usize,

    pub vote_start_interval: u64,

    pub spammer_interval: u64,

    pub endscreen_votetime: u64,
    pub endscreen_post_votetime: u64,

    pub vip_ad: String,
    pub vip_nom: String,

    pub announce_nominator: Option<bool>,
    pub vip_vote_weight: Option<usize>,

    pub animate: bool,
    pub animate_override: HashMap<String, bool>,

    pub options_minlen: usize,
    pub options_reserved_hidden: HashSet<String>,
    pub options_reserved_trie: HashSet<String>,
}

impl MapVoteConfig {
    pub fn from_json(other: MapVoteConfigJson) -> Self {
        Self {
            enabled: other.enabled,
            n_options: other.n_options,
            max_options: other.max_options,
            max_noms_per_vip: other.max_noms_per_vip,
            vote_start_interval: Duration::from_secs(other.vote_start_interval),
            spammer_interval: Duration::from_secs(other.spammer_interval),
            endscreen_votetime: Duration::from_secs(other.endscreen_votetime),
            endscreen_post_votetime: Duration::from_secs(other.endscreen_post_votetime),
            vip_nom: other.vip_ad,
            vip_ad: other.vip_nom,
            announce_nominator: other.announce_nominator,
            vip_vote_weight: other.vip_vote_weight,
            animate: other.animate,
            animate_override: other.animate_override.iter().map(|(k, v)| (k.clone().into_ascii_string().unwrap(), *v)).collect(),
            options_minlen: other.options_minlen,
            options_reserved_hidden: other.options_reserved_hidden,
            options_reserved_trie: other.options_reserved_trie,
        }
    }
}
