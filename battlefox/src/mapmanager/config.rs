use serde::{Serialize, Deserialize};

use super::PopState;
use crate::guard::{Judgement, SimpleJudgement};

#[derive(Debug, Clone, Copy)]
pub struct HasZeroPopState;
impl Judgement<Vec<PopState>> for HasZeroPopState {}
impl SimpleJudgement<Vec<PopState>> for HasZeroPopState {
    fn judge(about: &Vec<PopState>) -> Option<Self>
    where
        Self: Sized,
    {
        if about.iter().any(|x| x.min_players == 0) {
            Some(HasZeroPopState)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub enum MapManagerSetupError {
    /// pop states must contain one pop state with min_players = 0.
    PopState0Missing,
}

/// Convenience thing for loading stuff from Json.
#[derive(Debug, Serialize, Deserialize)]
struct MapManagerConfig {
    enabled: bool,
    pop_states: Vec<PopState>,

    vehicle_threshold: usize,
    leniency: usize,
}
