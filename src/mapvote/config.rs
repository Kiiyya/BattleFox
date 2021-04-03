use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MapVoteConfig {
    pub n_options: usize,
}
