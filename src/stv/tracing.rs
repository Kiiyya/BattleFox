//! Inspecting STV calculation step-by-step

use std::{collections::{HashMap, HashSet}, fmt::Display, marker::PhantomData, slice::Iter};
use std::fmt::Debug;
use std::hash::Hash;
use super::{Profile, Rat};
use itertools::Itertools;
use num_traits::{One, Zero};
use serde::{Serialize, Deserialize};

/// Visitor-like pattern.
#[allow(unused_variables)]
pub trait Tracer<A: Eq + Hash> {
    fn elem_t(&mut self, a: &A, b: &A, s: &Rat, profile_after: &Profile<A>) {}
    fn consume(&mut self, a: &A, profile_after: &Profile<A>) {}

    fn t_toall(&mut self, a: &A, s: &Rat, profile_after: &Profile<A>) {}

    fn electing(&mut self, alts: &HashSet<A>, profile_after: &Profile<A>) {}
    fn eliminating(&mut self, alt: &A, profile_after: &Profile<A>) {}

    /// When there's two or more alts tied for last place.
    fn reject_tie_break(&mut self, between: &HashSet<A>, chosen: &A, score: &Rat) {}
    fn stv_1winner_tiebreak(&mut self, between: &HashSet<A>, chosen: &A) {}
}

pub trait StatefulTracer<A> {
    type State;
    fn get_state(&self) -> Self::State;
}

pub trait LoggingTracer<'log, A> {
    type Item: 'log;
    type LogIter : Iterator<Item = &'log Self::Item>;
    fn log_iter(&'log self) -> Self::LogIter;
}

////////////////////////////////////

/// Disables tracing
pub struct NoTracer;
impl<A: Eq + Hash> Tracer<A> for NoTracer { }

////////////////////////////////////

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StvAction<A: Eq + Hash> {
    Starting(Profile<A>),
    ElemT {
        a: A,
        b: A,
        s: Rat,
        profile_afterwards: Profile<A>,
    },
    Consume {
        alt: A,
        profile_afterwards: Profile<A>,
    },

    ToAll {
        from: A,
        howmuch: Rat,
        profile_afterwards: Profile<A>,
    },

    Elected {
        elected: HashSet<A>,
        profile_afterwards: Profile<A>,
    },
    Eliminated {
        alt: A,
        profile_afterwards: Profile<A>,
    },

    RejectTiebreak {
        tied: HashSet<A>,
        chosen: A,
        score: Rat,
    },
    Stv1WinnerTiebreak {
        tied: HashSet<A>,
        chosen: A,
    },
}

impl<A: Eq + Hash> StvAction<A> {
    pub fn get_profile_after(&self) -> Option<&Profile<A>> {
        match self {
            StvAction::ToAll {
                from: _,
                howmuch: _,
                profile_afterwards,
            } => Some(profile_afterwards),
            StvAction::Consume {
                alt: _,
                profile_afterwards,
            } => Some(profile_afterwards),
            StvAction::RejectTiebreak {
                tied: _,
                chosen: _,
                score: _,
            } => None,
            StvAction::Elected {
                elected: _,
                profile_afterwards,
            } => Some(profile_afterwards),
            StvAction::Eliminated {
                alt: _,
                profile_afterwards,
            } => Some(profile_afterwards),
            StvAction::ElemT {
                a: _,
                b: _,
                s: _,
                profile_afterwards,
            } => Some(profile_afterwards),
            StvAction::Stv1WinnerTiebreak { tied: _, chosen: _ } => None,
            StvAction::Starting(profile) => {
                Some(profile)
            }
        }
    }
}

impl<A: Display + Debug + Eq + Hash> Display for StvAction<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StvAction::ToAll {
                from,
                howmuch,
                profile_afterwards: _,
            } => write!(f, "ToAll({}, {})", from, howmuch),
            StvAction::Consume {
                alt,
                profile_afterwards: _,
            } => write!(f, "Consume({})", alt),
            StvAction::RejectTiebreak {
                tied,
                chosen,
                score,
            } => write!(
                f,
                "RejectTieBreak({:?}, chosen: {}, score: {})",
                tied, chosen, score
            ),
            StvAction::Elected {
                elected,
                profile_afterwards: _,
            } => write!(f, "Elected({:?})", elected),
            StvAction::Eliminated {
                alt,
                profile_afterwards: _,
            } => write!(f, "Eliminated({})", alt),
            StvAction::ElemT {
                a,
                b,
                s,
                profile_afterwards: _,
            } => write!(f, "ElemT({}, {}, {})", a, b, s),
            StvAction::Stv1WinnerTiebreak { tied, chosen } => {
                write!(f, "Stv1WinnerTieBreak({:?}, {})", tied, chosen)
            }
            StvAction::Starting(_) => {
                write!(f, "Starting")
            }
        }
    }
}

/// Logs all events, even the very low-level ones.
#[derive(Debug, Serialize, Deserialize)]
pub struct DetailedTracer<A: Eq + Hash> {
    // invariant: self.trace.len() >= 1
    trace: Vec<StvAction<A>>,
    profile: Profile<A>,
}

impl<A: Clone + Eq + Hash> DetailedTracer<A> {
    pub fn start(profile: Profile<A>) -> Self {
        Self {
            trace: vec![StvAction::Starting(profile.clone())],
            profile
        }
    }
}

impl<A: Clone + Eq + Hash> Tracer<A> for DetailedTracer<A> {
    fn elem_t(&mut self, a: &A, b: &A, s: &Rat, profile_after: &Profile<A>) {
        self.trace.push(StvAction::ElemT {
            a: a.to_owned(),
            b: b.to_owned(),
            s: s.to_owned(),
            profile_afterwards: profile_after.to_owned(),
        })
    }

    fn electing(&mut self, alts: &HashSet<A>, profile_after: &Profile<A>) {
        self.trace.push(StvAction::Elected {
            elected: alts.to_owned(),
            profile_afterwards: profile_after.to_owned(),
        })
    }

    fn eliminating(&mut self, alt: &A, profile_after: &Profile<A>) {
        self.trace.push(StvAction::Eliminated {
            alt: alt.to_owned(),
            profile_afterwards: profile_after.to_owned(),
        })
    }

    fn t_toall(&mut self, a: &A, s: &Rat, profile_after: &Profile<A>) {
        self.trace.push(StvAction::ToAll {
            from: a.to_owned(),
            howmuch: s.to_owned(),
            profile_afterwards: profile_after.to_owned(),
        })
    }

    fn consume(&mut self, a: &A, profile_after: &Profile<A>) {
        self.trace.push(StvAction::Consume {
            alt: a.to_owned(),
            profile_afterwards: profile_after.to_owned(),
        })
    }

    fn reject_tie_break(&mut self, between: &HashSet<A>, chosen: &A, score: &Rat) {
        self.trace.push(StvAction::RejectTiebreak {
            tied: between.to_owned(),
            chosen: chosen.to_owned(),
            score: score.to_owned(),
        })
    }

    fn stv_1winner_tiebreak(&mut self, between: &HashSet<A>, chosen: &A) {
        self.trace.push(StvAction::Stv1WinnerTiebreak {
            tied: between.to_owned(),
            chosen: chosen.to_owned(),
        })
    }
}

impl <A: Clone + Eq + Hash> StatefulTracer<A> for DetailedTracer<A> {
    type State = Profile<A>;

    fn get_state(&self) -> Self::State {
        self.profile.clone()
    }
}

impl <'log, A: Clone + Eq + Hash + 'log> LoggingTracer<'log, A> for DetailedTracer<A> {
    type Item = StvAction<A>;
    type LogIter = Iter<'log, Self::Item>;

    fn log_iter(&'log self) -> Self::LogIter {
        self.trace.iter()
    }
}

#[derive(Debug)]
pub struct DuoTracer<A: Eq + Hash, T1: Tracer<A>, T2: Tracer<A>> {
    pub t1: T1,
    pub t2: T2,
    _a: PhantomData<A>,
}

impl <A: Eq + Hash, T1: Tracer<A>, T2: Tracer<A>> Tracer<A> for DuoTracer<A, T1, T2> {
    fn elem_t(&mut self, a: &A, b: &A, s: &Rat, profile_after: &Profile<A>) {
        self.t1.elem_t(a, b, s, profile_after);
        self.t2.elem_t(a, b, s, profile_after);
    }

    fn consume(&mut self, a: &A, profile_after: &Profile<A>) {
        self.t1.consume(a, profile_after);
        self.t2.consume(a, profile_after);
    }

    fn t_toall(&mut self, a: &A, s: &Rat, profile_after: &Profile<A>) {
        self.t1.t_toall(a, s, profile_after);
        self.t2.t_toall(a, s, profile_after);
    }

    fn electing(&mut self, alts: &HashSet<A>, profile_after: &Profile<A>) {
        self.t1.electing(alts, profile_after);
        self.t2.electing(alts, profile_after);
    }

    fn eliminating(&mut self, alt: &A, profile_after: &Profile<A>) {
        self.t1.eliminating(alt, profile_after);
        self.t2.eliminating(alt, profile_after);
    }

    fn reject_tie_break(&mut self, between: &HashSet<A>, chosen: &A, score: &Rat) {
        self.t1.reject_tie_break(between, chosen, score);
        self.t2.reject_tie_break(between, chosen, score);
    }

    fn stv_1winner_tiebreak(&mut self, between: &HashSet<A>, chosen: &A) {
        self.t1.stv_1winner_tiebreak(between, chosen);
        self.t2.stv_1winner_tiebreak(between, chosen);
    }
}

/// Distribution of where vote weight is allocated to.
///
/// 0 <= (sum of all branches) <= 1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Distr<A: Eq + Hash> {
    distr: HashMap<A, Rat>
}

impl <A: Eq + Hash + Clone> Distr<A> {
    pub fn single(a: A, weight: Rat) -> Self {
        Self {
            distr: hashmap!{
                a => weight
            }
        }
    }

    fn elem_t(&mut self, a: &A, b: &A, s: &Rat) {
        if let Some(x) = self.distr.get_mut(a) {
            // when s=0 ==> x = x * (1), so a no-op
            // when s=1 ==> x = x * (1-1), so delete fully
            let temp = (&*x) * s;
            *x *= Rat::one() - s;
            self.distr.insert(b.clone(), temp);
        }
    }

    fn consume(&mut self, a: &A) {
        self.distr.remove(a);
    }

    /// Assuming there exists at most one alternative with weight > 0:
    /// - get that alternative, or
    /// - return None, when all alternatives have 0 weight.
    ///
    /// # Panics
    /// - When more than one alternative has more than 0 weight.
    pub fn get_single(&self) -> Option<(&A, &Rat)> {
        let pos_w = self.distr.iter().filter(|(_, w)| w > &&Rat::zero()).collect_vec();

        if pos_w.len() > 1 {
            panic!("Distr::get_single(): at most one alternative can have weight > 0. If you are calling get_single() on a trace resulting from more than one seat (or containing elem_t with s != 1), then... don't, because it makes no sense.")
        }

        pos_w.get(0).copied()
    }
}

pub type Assignment<P, A> = HashMap<P, Distr<A>>;

#[derive(Debug, Serialize, Deserialize)]
pub struct AssignmentTracker<P: Eq + Hash, A: Eq + Hash> {
    assignment: Assignment<P, A>,
}

impl <P: Eq + Hash, A: Eq + Hash> AssignmentTracker<P, A> {
    pub fn start(assignment: Assignment<P, A>) -> Self {
        Self {
            assignment
        }
    }
}

impl<P: Eq + Hash, A: Eq + Hash + Clone> Tracer<A> for AssignmentTracker<P, A> {
    fn elem_t(&mut self, a: &A, b: &A, s: &Rat, _: &Profile<A>) {
        self.assignment.iter_mut().for_each(|(_, distr)| distr.elem_t(a, b, s));
    }

    fn consume(&mut self, a: &A, _: &Profile<A>) {
        self.assignment.iter_mut().for_each(|(_, distr)| distr.consume(a))
    }
}

impl <P: Clone + Eq + Hash, A: Clone + Eq + Hash> StatefulTracer<A> for AssignmentTracker<P, A> {
    type State = Assignment<P, A>;

    fn get_state(&self) -> Self::State {
        self.assignment.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HistEntry<P: Eq + Hash, A: Eq + Hash> {
    Starting {
        profile: Profile<A>,
        assignment: Assignment<P, A>,
    },
    Elim {
        a: A,
        profile: Profile<A>,
        assignment: Assignment<P, A>,
    },
    Elect {
        elected: HashSet<A>,
        profile: Profile<A>,
        assignment: Assignment<P, A>,
    },
}

// impl <P: Eq + Hash, A: Eq + Hash> HistEntry<P, A> {
//     pub fn get_profile(&self) -> &Profile<A> {
//         match self {
//             HistEntry::Starting { profile, assignment: _ } => profile,
//             HistEntry::Elim { profile, assignment: _ } => profile,
//             HistEntry::Elect { profile, assignment: _ } => profile,
//         }
//     }

//     pub fn get_assignment(&self) -> &Assignment<P, A> {
//         match self {
//             HistEntry::Starting { profile: _, assignment } => assignment,
//             HistEntry::Elim { profile: _, assignment } => assignment,
//             HistEntry::Elect { profile: _, assignment } => assignment,
//         }
//     }
// }

#[derive(Debug, Serialize, Deserialize)]
pub struct AnimTracer<P: Eq + Hash, A: Eq + Hash> {
    stvtracer: DetailedTracer<A>,
    assignment: AssignmentTracker<P, A>,

    /// # Invariant:
    ///
    /// len() >= 1
    assign_history: Vec<HistEntry<P, A>>,
}

impl <P: Clone + Eq + Hash, A: Clone + Eq + Hash> AnimTracer<P, A> {
    pub fn start(profile: Profile<A>, assignment: Assignment<P, A>) -> Self {
        Self {
            stvtracer: DetailedTracer::start(profile.clone()),
            assignment: AssignmentTracker::start(assignment.clone()),
            assign_history: vec![HistEntry::Starting{profile, assignment}],
        }
    }

    pub fn get_start(&self) -> (&Profile<A>, &Assignment<P, A>) {
        match &self.assign_history[0] { // [0] safe because len() >= 1 invariant.
            HistEntry::Starting { profile, assignment } => {
                (profile, assignment)
            }
            HistEntry::Elim { a: _, profile: _, assignment: _ } => {
                panic!("Inner invariant violated")
            }
            HistEntry::Elect { elected: _, profile: _, assignment: _ } => {
                panic!("Inner invariant violated")
            }
        }
    }
}

impl <P: Clone + Eq + Hash + Debug, A: Clone + Hash + Eq + Debug> Tracer<A> for AnimTracer<P, A> {
    fn elem_t(&mut self, a: &A, b: &A, s: &Rat, profile_after: &Profile<A>) {
        self.stvtracer.elem_t(a, b, s, profile_after);
        self.assignment.elem_t(a, b, s, profile_after);
        trace!("AnimTrace elem_t({:?}, {:?}): {:?}", a, b, self.assignment.get_state());
    }

    fn consume(&mut self, a: &A, profile_after: &Profile<A>) {
        self.stvtracer.consume(a, profile_after);
        self.assignment.consume(a, profile_after);
    }

    fn t_toall(&mut self, a: &A, s: &Rat, profile_after: &Profile<A>) {
        self.stvtracer.t_toall(a, s, profile_after);
        self.assignment.t_toall(a, s, profile_after);
    }

    fn electing(&mut self, alts: &HashSet<A>, profile_after: &Profile<A>) {
        self.stvtracer.electing(alts, profile_after);
        self.assignment.electing(alts, profile_after);
        self.assign_history.push(HistEntry::Elect {
            elected: alts.clone(),
            profile: profile_after.clone(),
            assignment: self.assignment.get_state(),
        });
    }

    fn eliminating(&mut self, alt: &A, profile_after: &Profile<A>) {
        self.stvtracer.eliminating(alt, profile_after);
        self.assignment.eliminating(alt, profile_after);
        self.assign_history.push(HistEntry::Elim {
            a: alt.clone(),
            profile: profile_after.clone(),
            assignment: self.assignment.get_state(),
        });
    }

    fn reject_tie_break(&mut self, between: &HashSet<A>, chosen: &A, score: &Rat) {
        self.stvtracer.reject_tie_break(between, chosen, score);
        self.assignment.reject_tie_break(between, chosen, score);
    }

    fn stv_1winner_tiebreak(&mut self, between: &HashSet<A>, chosen: &A) {
        self.stvtracer.stv_1winner_tiebreak(between, chosen);
        self.assignment.stv_1winner_tiebreak(between, chosen);
    }
}

impl <P: Clone + Eq + Hash, A: Clone + Eq + Hash> StatefulTracer<A> for AnimTracer<P, A> {
    type State = (Profile<A>, Assignment<P, A>);

    fn get_state(&self) -> Self::State {
        (self.stvtracer.get_state(), self.assignment.get_state())
    }
}

impl <'log, P: Clone + Eq + Hash + 'log, A: Clone + Eq + Hash + 'log> LoggingTracer<'log, A> for AnimTracer<P, A> {
    type Item = HistEntry<P, A>;
    type LogIter = Iter<'log, HistEntry<P, A>>;

    fn log_iter(&'log self) -> Self::LogIter {
        self.assign_history.iter()
    }
}

#[cfg(test)]
mod test {
    use crate::hashset;
    use crate::ballot;
    use super::*;

    #[test]
    fn distr() {
        let one = Rat::one();
        // let two = &one + &one;

        let mut distr = Distr {
            distr: hashmap!{
                "a" => one.clone(),
            },
        };

        distr.elem_t(&"a", &"b", &one);

        dbg!(&distr);
        assert_eq!(&"b", distr.get_single().unwrap().0);
    }

    #[test]
    fn anim_tracer() {
        let _profile = Profile {
            alts: hashset!["Wolf", "Fox", "Eagle", "Penguin"],
            ballots: vec![
                ballot![1, "Eagle"],
                ballot![1, "Eagle"],
                ballot![1, "Eagle"],
                ballot![1, "Wolf", "Fox", "Eagle"],
                ballot![1, "Fox", "Wolf", "Eagle"],
                ballot![1, "Wolf", "Fox", "Eagle"],
                ballot![1, "Wolf", "Fox"],
            ],
        };

        // let at = AnimTracer::start(profile, assignment)
    }
}
