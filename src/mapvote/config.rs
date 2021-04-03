use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MapVoteConfig {
    pub n_options: usize,

    /// Not even VIPs will be able to do more than this
    pub max_options: usize,

    pub max_noms_per_vip: usize,
}
