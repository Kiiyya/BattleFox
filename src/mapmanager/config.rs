use super::PopState;
use crate::guard::{Judgement, SimpleJudgement};

#[derive(Debug, Clone, Copy)]
pub struct HasZeroPopState;
impl<E: Eq + Clone> Judgement<Vec<PopState<E>>> for HasZeroPopState {}
impl<E: Eq + Clone> SimpleJudgement<Vec<PopState<E>>> for HasZeroPopState {
    fn judge(about: &Vec<PopState<E>>) -> Option<Self>
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
