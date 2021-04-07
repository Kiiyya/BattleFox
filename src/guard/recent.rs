use super::{or::Or, Guard, InferFrom, Judgement};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub enum Age<J> {
    Recent(J),
    Old,
}

#[derive(Debug, Clone, Copy)]
pub struct Recent<J> {
    timestamp: Instant,
    judgement: Age<J>,
}

impl<J> Recent<J> {
    pub fn now(j: J) -> Self {
        Self {
            timestamp: Instant::now(),
            judgement: Age::Recent(j),
        }
    }
}

impl<J: MaxAge> Recent<J> {
    pub fn is_recent(&self) -> bool {
        self.timestamp.elapsed() < J::MAX_AGE
    }

    pub fn and_then<T>(self, f: impl FnOnce(J) -> T) -> Option<T> {
        match self.cases() {
            Age::Recent(j) => Some(f(j)),
            Age::Old => None,
        }
    }
}

impl<T, J: Judgement<T>> Guard<T, Recent<J>> {
    pub fn age(&self) -> Duration {
        self.judgement.timestamp.elapsed()
    }
}

pub trait MaxAge {
    const MAX_AGE: Duration;
}

impl<A, B> MaxAge for Or<A, B>
where
    A: MaxAge,
    B: MaxAge,
{
    // minimum of the two (pessimistic).
    const MAX_AGE: Duration = if A::MAX_AGE.as_secs() < B::MAX_AGE.as_secs()
        || A::MAX_AGE.as_secs() == B::MAX_AGE.as_secs()
            && A::MAX_AGE.as_nanos() < B::MAX_AGE.as_nanos()
    {
        A::MAX_AGE
    } else {
        B::MAX_AGE
    };
}

impl<J: MaxAge> Recent<J> {
    pub fn infer<Ret>(self, rule: impl FnOnce(J) -> Ret) -> Recent<Ret> {
        Recent {
            timestamp: self.timestamp,
            judgement: match self.judgement {
                Age::Recent(j) => Age::Recent(rule(j)),
                Age::Old => Age::Old,
            },
        }
    }

    pub fn cases(self) -> Age<J> {
        if let Age::Recent(j) = self.judgement {
            if self.timestamp.elapsed() < J::MAX_AGE {
                Age::Recent(j)
            } else {
                Age::Old
            }
        } else {
            Age::Old
        }
    }
}

impl<T, FromJ, TargetJ> InferFrom<T, Recent<FromJ>> for Recent<TargetJ>
where
    FromJ: Judgement<T> + MaxAge,
    TargetJ: Judgement<T> + InferFrom<T, FromJ>,
{
    fn infer(from: Recent<FromJ>) -> Self {
        from.infer(TargetJ::infer)
    }
}

impl<T, J: Judgement<T> + MaxAge> Guard<T, Recent<J>> {
    // pub fn fork_recent<Ret>(self, recent: impl FnOnce(Guard<T, &J>) -> Ret) -> Age<Ret>
    //     where J: MaxAge
    // {
    //     match self.judgement.cases() {
    //         Age::Recent(j) => {
    //             Age::Recent()
    //         }
    //         Age::Old => Age::Old
    //     }
    //     todo!()
    // }

    // pub fn fork<Ret, FRecent, FOld>(self, recent: FRecent, old: FOld) -> Ret
    // where
    //     FRecent: FnOnce(&Guard<T, J>) -> Ret,
    //     FOld: FnOnce() -> Ret,
    // {
    //     if self.judgement.timestamp.elapsed() < J::MAX_AGE {
    //         recent(&Guard {
    //             inner: self.inner,
    //             judgement: self.judgement.judgement,
    //         })
    //     } else {
    //         old()
    //     }
    // }

    pub fn infer_recent<TargetJ: Judgement<T>>(
        self,
        rule: impl FnOnce(J) -> TargetJ,
    ) -> Guard<T, Recent<TargetJ>> {
        Guard {
            inner: self.inner,
            judgement: self.judgement.infer(rule),
        }
    }

    pub fn auto_recent<TargetJ>(self) -> Guard<T, Recent<TargetJ>>
    where
        TargetJ: InferFrom<T, J>,
    {
        Guard {
            inner: self.inner,
            judgement: self.judgement.infer(TargetJ::infer),
        }
    }

    // pub fn f<U, J2, FRecent, FOld>(&self, recent: FRecent, old: FOld) -> Guard<U, Recent<J2>>
    // where
    //     FRecent: FnOnce(&J) -> Guard<U, Recent<J2>>,
    //     FOld: FnOnce() -> Guard<U, Recent<J2>>,
    //     J2: Judgement<U>,
    // {
    //     if self.timestamp.elapsed() < J::MAX_AGE {
    //         recent(&self.judgement)
    //     } else {
    //         old()
    //     }
    // }

    pub fn cases(self) -> Age<Guard<T, J>> {
        match self.judgement.cases() {
            Age::Recent(j) => Age::Recent(Guard {
                inner: self.inner,
                judgement: j,
            }),
            Age::Old => Age::Old,
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

///////////////////////////////////////////////////////

// pub trait Home {}

// pub trait Resolver<T, J: Judgement<T>> {

// }

// pub struct RGuard<T, J: Judgement<T>, R: Resolver<T, J>> {
//     home: ,
//     t: PhantomData<T>,
//     j: PhantomData<J>
// }

// impl<T, J: Judgement<T>> RGuard<T, J> {
//     pub fn try_get(&self) -> Option<Guard<T, Recent<J>>> {
//         todo!()
//     }

//     pub async fn get(&self) -> Guard<T, Recent<J>> {
//         todo!()
//     }

//     pub fn using<Ret>(&self, f: impl FnOnce(Guard<T, J>) -> Ret) -> Ret {
//         todo!()
//     }

//     // pub async fn using_async<F, Fut, Ret>(&self, f: F) -> Ret
//     //     where
//     //         F: FnOnce(Guard<T, J>) -> Fut,
//     //         Fut: Future<Output = Ret>,
//     // {
//     //     todo!()
//     // }
// }
