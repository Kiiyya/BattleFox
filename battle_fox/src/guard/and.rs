use super::{Judgement, SimpleJudgement};

/// Instances of this type are proofs which express that `A` *and* `B` hold.
///
/// - You introduce `A and B` with `And::and(a, b)`.
/// - You obtain just `A` from `A and B` via `my_and.left()`.
#[derive(Debug)]
pub struct And<A, B>(A, B);
impl<A, B> And<A, B> {
    /// Constructs a a proof that both `A` and `B` hold.
    pub fn and(p1: A, p2: B) -> And<A, B> {
        And(p1, p2)
    }
}
impl<A, B> And<A, B> {
    /// When you have `A` and `B`, then you also have `A`.
    pub fn left(self) -> A {
        self.0
    }
    /// When you have `A` and `B`, then you also have `B`.
    pub fn right(self) -> B {
        self.1
    }
}
impl<A: Clone, B: Clone> Clone for And<A, B> {
    fn clone(&self) -> Self {
        And(self.0.clone(), self.1.clone())
    }
}
impl<A: Copy, B: Copy> Copy for And<A, B> {}

impl<T, A, B> Judgement<T> for And<A, B> {}
impl<T, A, B> SimpleJudgement<T> for And<A, B>
where
    A: SimpleJudgement<T>,
    B: SimpleJudgement<T>,
{
    fn judge(about: &T) -> Option<Self>
    where
        Self: Sized,
    {
        if let Some(a) = A::judge(about) {
            B::judge(about).map(|b| Self(a, b))
        } else {
            None
        }
    }
}
