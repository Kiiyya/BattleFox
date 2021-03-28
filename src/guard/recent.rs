use std::time::{Duration, Instant};

use super::{Guard, Judgement};

pub struct Recent<J> {
    timestamp: Instant,
    judgement: J,
}

/// Age in milliseconds.
pub struct MaxAge<J, const AGE: u64>(J);

// macro_rules! MaxAge {
//     ($j:ty,) => {
//     };
// }

impl <T, J: Judgement<T>> Guard<T, Recent<J>> {
    pub fn elapsed(&self) -> Duration {
        self.judgement.timestamp.elapsed()
    }
}

impl<T, J: Judgement<T>> Judgement<T> for Recent<J> { }

#[cfg(test)]
mod test {
    #[test]
    fn test() {

    }
}
