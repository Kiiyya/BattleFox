use std::time::{Duration, Instant};

use super::{Cases, Guard, Judgement, or::Or};

pub trait Resolver {}

pub trait Resolvable {}

#[derive(Debug, Clone)]
pub struct Recent<J> {
    timestamp: Instant,
    judgement: J,
}

impl<T, J: Judgement<T>> Guard<T, Recent<J>> {
    pub fn age(&self) -> Duration {
        self.judgement.timestamp.elapsed()
    }
}

pub enum Age<J> {
    Recent(J),
    Old,
}

pub trait MaxAge {
    const MAX_AGE: Duration;
}

// impl <A: MaxAge, B: MaxAge> MaxAge for Or<A, B> {
//     const MAX_AGE: Duration = if true { Duration::from_secs(1) } else { Duration::from_secs(2) };
// }

impl<J: MaxAge> Cases for Recent<J> {
    type Cases = Age<J>;

    fn cases(self) -> Self::Cases {
        if self.timestamp.elapsed() < J::MAX_AGE {
            Age::Recent(self.judgement)
        } else {
            Age::Old
        }
    }
}

// if J is a judgement about T, then Recent<J> is a judgement about T as well.
impl<T, J: Judgement<T>> Judgement<T> for Recent<J> {}

#[cfg(test)]
mod test {
    #[test]
    fn test() {}
}
