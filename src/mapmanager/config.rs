use super::PopState;
use crate::guard::{Guard, Judgement};

#[derive(Debug, Clone, Copy)]
pub struct HasZeroPopState;
impl Judgement<Vec<PopState>> for HasZeroPopState {}

#[derive(Debug)]
pub enum MapManagerSetupError {
    /// pop states must contain one pop state with min_players = 0.
    PopState0Missing,
}

// impl Guard<Vec<PopState>, HasZeroPopState> {
//     /// Validate pop_states
//     pub fn guard_zeropop(pop_states: Vec<PopState>) -> Result<Self, MapManagerSetupError> {
//         if pop_states.iter().any(|x| x.min_players == 0) {
//             Ok(
//                 Self {
//                     inner: pop_states,
//                     judgement: HasZeroPopState,
//                 }
//             )
//         } else {
//             Err(MapManagerSetupError::PopState0Missing)
//         }
//     }
// }

/// Validate pop_states
pub fn guard_zeropop(
    pop_states: Vec<PopState>,
) -> Result<Guard<Vec<PopState>, HasZeroPopState>, MapManagerSetupError> {
    if pop_states.iter().any(|x| x.min_players == 0) {
        Ok(Guard::new(pop_states, HasZeroPopState))
    } else {
        Err(MapManagerSetupError::PopState0Missing)
    }
}
