/*!
Enforcing contracts at (mostly) compile-time.

Having an *instance* of a type amounts to having a *proof* of that type.

A key part of what makes this work is setting anything that can make your system
unsound to private. Only expose some constructors or implications/lemmas which
are "sound" in some way which you define.

# Example
The idea is that `ban` can only be invoked by someone who has an instance of
`Guard<Player, Admin>`, which can only be construcuted with
private constructors, so you have some degree of compile-
time verified assurances. At least you'll be less likely
to slip up.

```
# use seamless::guard::{Judgement, Guard};
# #[derive(Debug)]
struct Player { name: String, };

# #[derive(Debug)]
struct Admin; // Zero-Sized-Type!
impl Judgement<Player> for Admin {}

fn ban(actor: Guard<Player, Admin>, target: Player) {
    println!("{:?} banned {:?}!", *actor, target);
    // ...
}

```
# Example: Implications, `infer`
```
# use seamless::guard::{Judgement, Guard};
# #[derive(Debug, Clone)]
struct Admin;
impl Judgement<String> for Admin {}

# #[derive(Debug, Clone)]
struct Mod;
impl Judgement<String> for Mod {}

// functions are implications.
// This is axiomatic. You can write bullshit here and make it unsound.
// Admin(T) ==> Moderator(T).
fn admin_is_mod(_admin: Admin) -> Mod {
    Mod
}

fn kick(actor: Guard<String, Mod>, target: &str) {
    // we can assume that the player is an admin, otherwise this function
    // wouldn't even be callable
    println!("{} just kicked {}!", *actor, target);
}

fn mymain(admin_player: Guard<String, Admin>) {
    // You can't just make a guard yourself, need to use proper methods for that.
    // let admin_player : Guard<String, Admin> = Guard {
    //    inner: String::new(), // error: private field.
    //    judgement: Admin, // error: private field.
    // };

    // kick(admin_player); // uh oh, expected Mod, but we have Admin!

    // Easy, just solve it with our inferrence rule `admin_is_mod`.
    // Note that while `admin_is_mod` does potentially unsound stuff,
    // we as API consumers do not have access to the constructor of
    // `Admin` or `Mod`.
    let moderator_player = admin_player.infer(admin_is_mod);
    kick(moderator_player, "Kiiya");
}
```
*/

use std::ops::{Deref, DerefMut};

pub mod and;
pub mod or;
pub mod recent;

pub trait Cases {
    type Cases;
    fn cases(self) -> Self::Cases;
}

pub trait AutoInfer<FromJ> {
    fn infer(from: FromJ) -> Self;
}

pub trait Complement<J> {
    type Complement;
}

/// Marker trait. Asserts that a value of type `T` fulfills some arbitrary condition.
pub trait Judgement<T> {}

pub trait SimpleJudgement<T>: Judgement<T> {
    fn judge(about: &T) -> Option<Self>
    where
        Self: Sized;
}

pub struct True;
pub struct False;

/// A wrapper around a `T`, with a proof attached that it fulfills some arbitrary condition.
/// For example, that a player is an admin, or that a number is even, etc.
///
/// Often, the attached proofs/judgements will be zero-sized (e.g. `And`, `Admin`, ...).
///
/// Each `Or` adds one bit of extra information though, other judgements may
/// add more too.
#[derive(Debug)]
pub struct Guard<T, J: Judgement<T>> {
    inner: T,
    judgement: J,
}

impl<T, J: Judgement<T>> Guard<T, J> {
    /// Consumes the `Guard` and returns the unpacked inner value.
    pub fn get(self) -> T {
        self.inner
    }
}

impl<T: Clone, J: Judgement<T> + Clone> Clone for Guard<T, J> {
    fn clone(&self) -> Self {
        Guard {
            inner: self.inner.clone(),
            judgement: self.judgement.clone(),
        }
    }
}
impl<T: Copy, J: Judgement<T> + Copy> Copy for Guard<T, J> {}

impl<T, J: Judgement<T>> Guard<T, J> {
    pub fn new(inner: T) -> Option<Self>
    where
        J: SimpleJudgement<T>,
    {
        J::judge(&inner).map(|judgement| Self { inner, judgement })
    }

    /// # Safety
    /// You need to make sure yourself that the judgement fits.
    pub unsafe fn new_raw(inner: T, judgement: J) -> Self {
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

    pub fn auto<TargetJ: Judgement<T>>(self) -> Guard<T, TargetJ>
    where
        TargetJ: AutoInfer<J>,
    {
        Guard {
            inner: self.inner,
            judgement: TargetJ::infer(self.judgement),
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
        let admin_player = unsafe { Guard::new_raw(player, Admin::<()>(PhantomData)) };

        // // kick(admin_player); // uh oh, expected Mod, but we have Admin!
        // let or: Or<_, _> =

        let moderator_player = admin_player.infer(admin_is_mod);
        kick(moderator_player);
    }
}
