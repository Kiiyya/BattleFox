//! Enforcing contracts at (mostly) compile-time.
//!
//! Having an *instance* of a type amounts to having a *proof* of that type.
//!
//! A key part of what makes this work is setting anything that can make your system
//! unsound to private. Only expose some constructors or implications/lemmas which
//! are "sound" in some way which you define.
//!
//! # Example
//! The idea is that `ban` can only be invoked by someone who has an instance of
//! `Guard<Player, Admin>`, which can only be construcuted with
//! private constructors, so you have some degree of compile-
//! time verified assurances. At least you'll be less likely
//! to slip up.
//!
//! ```
//! struct Player { name: String, };
//!
//! #[derive(Clone)]
//! struct Admin; // Zero-Sized-Type!
//! impl Judgement<Player> for Admin {}
//!
//! fn ban(actor: Guard<Player, Admin>, target: Player) {
//!     println!("{} banned {}!", *actor, target);
//!     // ...
//! }
//!
//! ```
//! # Example: Implications, `infer`
//! ```
//! # use super::{Guard, Judgement};
//! # #[derive(Debug, Clone)]
//! struct Admin;
//! impl Judgement<String> for Admin {}
//!
//! # #[derive(Debug, Clone)]
//! struct Mod;
//! impl Judgement<String> for Mod {}
//!
//! // functions are implications.
//! // This is axiomatic. You can write bullshit here and make it unsound.
//! // Admin(T) ==> Moderator(T).
//! fn admin_is_mod(_admin: Admin) -> Mod {
//!     Mod
//! }
//!
//! fn kick(actor: Guard<String, Mod>, target: String) {
//!     // we can assume that the player is an admin, otherwise this function
//!     // wouldn't even be callable
//!     println!("{} just kicked {}!", *actor, target);
//! }
//!
//! fn main() {
//!     let admin_player : Guard<String, Admin>;
//!
//!     // kick(admin_player); // uh oh, expected Mod, but we have Admin!
//!
//!     // Easy, just solve it with our inferrence rule `admin_is_mod`.
//!     // Note that while `admin_is_mod` does potentially unsound stuff,
//!     // we as API consumers do not have access to the constructor of
//!     // `Admin` or `Mod`.
//!     let moderator_player = admin_player.infer(admin_is_mod);
//!     kick(moderator_player);
//! }
//!```

use std::ops::{Deref, DerefMut};

use either::Either;

pub trait Cases {
    type Cases;

    fn cases(self) -> Self::Cases;
}

/// Instances of this type are proofs which express that `A` *and* `B` hold.
/// 
/// - You introduce `A and B` with `And::and(a, b)`.
/// - You obtain just `A` from `A and B` via `my_and.left()`.
/// 
/// # Example
/// ```
/// Guard<Player, And<Admin, Mod>>
/// ```
pub struct And<A, B>(A, B);
impl <A, B> And<A, B> {
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

/// Instances of this type are proofs which express that `A` *or* `B` hold.
/// # Example
/// ```
/// Guard<Player, Or<Admin, SomeOtherJudgementIdk>>
/// ```
pub struct Or<A, B>(Either<A, B>);

impl<A, B> Or<A, B> {
    pub fn fork<Target>(
        self,
        left: impl FnOnce(A) -> Target,
        right: impl FnOnce(B) -> Target,
    ) -> Target {
        match self.0 {
            Either::Left(p1) => left(p1),
            Either::Right(p2) => right(p2),
        }
    }

    // pub fn cases(self) -> Either<A, B> {
    //     self.0
    // }

    /// Constructs a proof of `Or<A, B>` from one branch.
    /// If you know that `A` is true, then you know that `A or B` is true.
    pub fn left(p1: A) -> Or<A, B> {
        Or(Either::Left(p1))
    }
    /// Constructs a proof of `Or<A, B>` from one branch.
    /// If you know that `B` is true, then you know that `A or B` is true.
    pub fn right(p2: B) -> Or<A, B> {
        Or(Either::Right(p2))
    }
}

impl <A, B> Cases for Or<A, B> {
    type Cases = Either<A, B>;

    fn cases(self) -> Self::Cases {
        self.0
    }
}

impl<A: Clone, B: Clone> Clone for Or<A, B> {
    fn clone(&self) -> Self {
        Or(match &self.0 {
            Either::Left(l) => Either::Left(l.clone()),
            Either::Right(r) => Either::Right(r.clone()),
        })
    }
}
impl<A: Copy, B: Copy> Copy for Or<A, B> {}

// pub fn cases<A, B, Target>(
//     left: impl FnOnce(A) -> Target,
//     right: impl FnOnce(B) -> Target,
// ) -> impl FnOnce(Or<A, B>) -> Target {
//     |or: Or<A, B>| match or.0 {
//         Either::Left(p1) => left(p1),
//         Either::Right(p2) => right(p2),
//     }
// }

/// Marker trait. Asserts that a value of type `T` fulfills some arbitrary condition.
pub trait Judgement<T> {}

impl<T, A: Judgement<T>, B: Judgement<T>> Judgement<T> for And<A, B> {}
impl<T, A: Judgement<T>, B: Judgement<T>> Judgement<T> for Or<A, B> {}

/// A wrapper around a `T`, with a proof attached that it fulfills some arbitrary condition.
/// For example, that a player is an admin, or that a number is even, etc.
/// 
/// Often, the attached proofs/judgements will be zero-sized (e.g. `And`, `Admin`, ...).
/// 
/// Each `Or` adds one bit of extra information though, other judgements may
/// add more too.
pub struct Guard<T, J: Judgement<T>> {
    inner: T,
    judgement: J,
}

impl<T: Clone, J: Judgement<T>> Clone for Guard<T, J> {
    fn clone(&self) -> Self {
        Guard {
            inner: self.inner.clone(),
            judgement: self.judgement.clone(),
        }
    }
}
impl<T: Copy, J: Judgement<T> + Copy> Copy for Guard<T, J> {}

impl<T, J: Judgement<T>> Guard<T, J> {
    pub fn new(inner: T, judgement: J) -> Self {
        Self { inner, judgement }
    }

    pub fn infer<TargetJ: Judgement<T>>(
        self,
        rule: impl FnOnce(J) -> TargetJ,
    ) -> Guard<T, TargetJ> {
        Guard {
            inner: self.inner,
            judgement: rule(self.judgement),
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

impl<T, J: Judgement<T>> Deref for Guard<T, J> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl<T, J: Judgement<T>> DerefMut for Guard<T, J> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

// pub mod dynamic {
//     //! When you want to create new Judgement types at runtime.
    
//     pub struct SomeJudgement<T> {

//     }
// }

// pub mod leniency {
//     //! To permit cached values, e.g. someone's admin may have expired, but
//     //! we still want to accept it within the last 10 seconds or so.
    
    

// }

pub mod enum_subset {
    use super::Judgement;

    pub trait EnumSubset<E> {

    }

    #[derive(Debug, Clone)]
    pub struct Subset<E> {

    }
    impl <E: Clone> Judgement<E> for Subset<E> {}
}

/////////////////////////////////////////////////////

#[cfg(test)]
mod test {
    use std::marker::PhantomData;

    use super::{Guard, Judgement};

    #[derive(Debug, Clone)]
    struct Admin<Server: Clone>(PhantomData<Server>);
    impl<Server: Clone> Judgement<String> for Admin<Server> {}

    #[derive(Debug, Clone)]
    struct Mod<Server: Clone>(PhantomData<Server>);
    impl<Server: Clone> Judgement<String> for Mod<Server> {}

    // functions are implications.
    // This is axiomatic. You can write bullshit here and make it unsound.
    // Admin(T, server) ==> Moderator(T, server).
    fn admin_is_mod<Server: Clone>(_admin: Admin<Server>) -> Mod<Server> {
        Mod(PhantomData)
    }

    fn kick<Server: Clone>(actor: Guard<String, Mod<Server>>) {
        // we can assume that the player is an admin, otherwise this function
        // wouldn't even be callable
        println!("{} is some kind of admin!", *actor);
    }

    #[test]
    fn main() {
        let player = String::new();
        let admin_player = Guard::new(player, Admin::<()>(PhantomData));

        // // kick(admin_player); // uh oh, expected Mod, but we have Admin!
        // let or: Or<_, _> =

        let moderator_player = admin_player.infer(admin_is_mod);
        kick(moderator_player);
    }
}
