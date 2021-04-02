use either::Either::{self, Left, Right};

use super::{Cases, Guard, Judgement, SimpleJudgement};

/// Instances of this type are proofs which express that `A` *or* `B` hold.
#[derive(Debug)]
pub struct Or<A, B>(Either<A, B>);

impl<A, B> Or<A, B> {
    pub fn fork<Target>(
        self,
        left: impl FnOnce(A) -> Target,
        right: impl FnOnce(B) -> Target,
    ) -> Target {
        match self.0 {
            Left(p1) => left(p1),
            Right(p2) => right(p2),
        }
    }

    /// Constructs a proof of `Or<A, B>` from one branch.
    /// If you know that `A` is true, then you know that `A or B` is true.
    pub fn left(p1: A) -> Or<A, B> {
        Or(Left(p1))
    }
    /// Constructs a proof of `Or<A, B>` from one branch.
    /// If you know that `B` is true, then you know that `A or B` is true.
    pub fn right(p2: B) -> Or<A, B> {
        Or(Right(p2))
    }
}

pub fn or_assoc<A, B, C>(j: Or<A, Or<B, C>>) -> Or<Or<A, B>, C> {
    match j.cases() {
        Left(a) => Or::left(Or::left(a)),
        Right(right) => match right.cases() {
            Left(b) => Or::left(Or::right(b)),
            Right(c) => Or::right(c),
        },
    }
}

pub fn or_assoc2<A, B, C>(j: Or<Or<A, B>, C>) -> Or<A, Or<B, C>> {
    match j.cases() {
        Left(left) => match left.cases() {
            Left(a) => Or::left(a),
            Right(b) => Or::right(Or::left(b)),
        },
        Right(c) => Or::right(Or::right(c)),
    }
}

pub fn or_comm<A, B>(j: Or<A, B>) -> Or<B, A> {
    match j.cases() {
        Left(a) => Or::right(a),
        Right(b) => Or::left(b),
    }
}

impl<A, B> Cases for Or<A, B> {
    type Cases = Either<A, B>;

    fn cases(self) -> Self::Cases {
        self.0
    }
}

impl<A: Clone, B: Clone> Clone for Or<A, B> {
    fn clone(&self) -> Self {
        Or(match &self.0 {
            Left(l) => Left(l.clone()),
            Right(r) => Right(r.clone()),
        })
    }
}
impl<A: Copy, B: Copy> Copy for Or<A, B> {}

impl<T, A, B> Judgement<T> for Or<A, B> {}
impl<T, A, B> SimpleJudgement<T> for Or<A, B>
where
    A: SimpleJudgement<T>,
    B: SimpleJudgement<T>,
{
    fn judge(about: &T) -> Option<Self>
    where
        Self: Sized,
    {
        if let Some(a) = A::judge(about) {
            Some(Or(Left(a)))
        } else {
            B::judge(about).map(|b| Or(Right(b)))
        }
    }
}

impl<T, L: Judgement<T>, R: Judgement<T>> Guard<T, Or<L, R>> {
    pub fn left(l: Guard<T, L>) -> Guard<T, Or<L, R>> {
        Guard {
            inner: l.inner,
            judgement: Or::left(l.judgement),
        }
    }

    pub fn right(r: Guard<T, R>) -> Guard<T, Or<L, R>> {
        Guard {
            inner: r.inner,
            judgement: Or::right(r.judgement),
        }
    }
}

impl<T, A: Judgement<T>, B: Judgement<T>> Guard<T, Or<A, B>> {
    pub fn fork<TargetJ: Judgement<T>>(
        self,
        left: impl FnOnce(Guard<T, A>) -> Guard<T, TargetJ>,
        right: impl FnOnce(Guard<T, B>) -> Guard<T, TargetJ>,
    ) -> Guard<T, TargetJ> {
        match self.judgement.0 {
            Either::Left(j) => left(Guard {
                inner: self.inner,
                judgement: j,
            }),
            Either::Right(j) => right(Guard {
                inner: self.inner,
                judgement: j,
            }),
        }
    }

    // pub fn cases(self) -> Either<Guard<T, A>, Guard<T, B>> {
    //     match self.judgement.cases() {
    //         Either::Left(l) => Either::Left(Guard {
    //             inner: self.inner,
    //             judgement: l,
    //         }),
    //         Either::Right(r) => Either::Right(Guard {
    //             inner: self.inner,
    //             judgement: r,
    //         }),
    //     }
    // }
}

impl<T, A: Judgement<T>, B: Judgement<T>> Cases for Guard<T, Or<A, B>> {
    type Cases = Either<Guard<T, A>, Guard<T, B>>;

    fn cases(self) -> Self::Cases {
        match self.judgement.cases() {
            Either::Left(l) => Either::Left(Guard {
                inner: self.inner,
                judgement: l,
            }),
            Either::Right(r) => Either::Right(Guard {
                inner: self.inner,
                judgement: r,
            }),
        }
    }
}
