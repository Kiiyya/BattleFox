//! Inspecting STV calculation step-by-step

use std::{collections::{HashMap, HashSet}, fmt::Display, marker::PhantomData, slice::Iter};
use std::fmt::Debug;
use std::hash::Hash;
use super::{Ballot, Profile, Rat};
use itertools::Itertools;
use num_traits::Zero;
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
    profile: Profile<A>,
}

impl <A: Eq + Hash + Clone + Debug> PartialEq for Distr<A> {
    fn eq(&self, other: &Self) -> bool {
        self.scores() == other.scores()
    }
}

impl <A: Eq + Hash + Clone + Debug> Distr<A> {
    pub fn scores(&self) -> HashMap<A, Rat> {
        self.profile.scores()
    }

    pub fn from_ballot(ballot: Ballot<A>) -> Self {
        Self {
            profile: Profile {
                alts: ballot.preferences.iter().cloned().collect(),
                ballots: vec![ballot],
            }
        }
    }

    fn elem_t(&mut self, a: &A, b: &A, s: &Rat) {
        self.profile = self.profile.elem_t(a, b, s, &mut NoTracer);
    }

    fn consume(&mut self, a: &A) {
        self.profile = self.profile.consume(a, &mut NoTracer);
    }

    /// Assuming there exists at most one alternative with weight > 0:
    /// - [Some] (Alternative)
    /// - return [None], when all alternatives have 0 weight
    /// - return [None] and log error when there's two alts with >0 weight.
    pub fn get_single(&self) -> Option<(A, Rat)> {
        let scores = self.scores();
        let scores = scores.iter().filter(|(_, score)| score > &&Rat::zero()).collect_vec();
        if scores.len() <= 1 {
            scores.get(0).map(|(a, r)| ((*a).clone(), (*r).clone()))
        } else {
            error!("Distr::get_single({:?}): scores.len() >= 2", self);
            None
        }
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

impl<P: Eq + Hash, A: Eq + Hash + Clone + Debug> Tracer<A> for AssignmentTracker<P, A> {
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
        // trace!("AnimTracer::elem_t({:?}, {:?}, {}) ==> {:#?}", a, b, s, self.get_state());
    }

    fn consume(&mut self, a: &A, profile_after: &Profile<A>) {
        self.stvtracer.consume(a, profile_after);
        self.assignment.consume(a, profile_after);
        // trace!("AnimTracer::consume({:?}) ==> {:#?}", a, self.get_state());
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
    use num_traits::One;

    #[test]
    fn distr() {
        let one = Rat::one();
        // let two = &one + &one;

        let mut distr = Distr::from_ballot(Ballot {
            weight: Rat::one(),
            preferences: vec!["a", "b"]
        });

        distr.elem_t(&"a", &"c", &one);
        assert_eq!("a", distr.get_single().unwrap().0);
        distr.elem_t(&"a", &"b", &one);

        dbg!(&distr);
        assert_eq!("b", distr.get_single().unwrap().0);
    }

    #[test]
    fn distr2() {
        // simple_logger::init();

        let one = Rat::one();
        let two = &one + &one;

        let dummy_profile = Profile { alts: HashSet::new(), ballots: Vec::new()};

        // let mut at = AnimTracer::start(dummy_profile.clone(), hashmap!{
        //     "kiiya" => Distr { distr: hashmap! {
        //         "z_night" => two,
        //     }}
        // });

        let distr = Distr::from_ballot(Ballot {
            weight: two,
            preferences: vec!["z_night"]
        });

        let mut at = AnimTracer::start(dummy_profile.clone(), hashmap! {
            "kiiya" => distr
        });

        at.elem_t(&"z_night", &"pearl", &Rat::zero(), &dummy_profile);
        at.elem_t(&"z_night", &"firestorm", &Rat::one(), &dummy_profile);
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

    /// https://github.com/Kiiyya/BattleFox/issues/18
    #[test]
    fn issue18_bad_personal_ballot_assignment() {
        use battlefield_rcon::bf4::Map;
        use Map::{Caspian as c, Propaganda as p, Shanghai as s, Locker as l};
        let profile = Profile {
            alts: hashset!{c, p, s, l},
            ballots: vec![
                ballot![1, s],
                ballot![1, s],
                ballot![1, c],
                ballot![1, l],
                ballot![1, p, s, c],
                ballot![1, s, l],
                ballot![1, l],
                ballot![1, p],
                ballot![1, s],
                ballot![1, s],
                ballot![1, l, p],
                ballot![2, c, s, p, l],
                ballot![1, l],
                ballot![1, s, c, l, p],
                ballot![1, l],
            ]
        };

        // the issue is nondeterministic, so try a couple times.
        for _ in 0..20 {
            type Player = usize;
            let assignment : HashMap<Player, _> = profile.ballots.iter()
                .map(|ballot| Distr::from_ballot(ballot.clone()))
                .enumerate()
                .collect();

            let mut tracer = AnimTracer::start(profile.clone(), assignment);

            let _ = profile.vanilla_stv_1(&mut tracer);

            // dbg!(&tracer.assign_history);
            match &tracer.assign_history[2] {
                HistEntry::Elim { a: _, profile: _, assignment } => {
                    assert_eq!(s, assignment.get(&11).unwrap().get_single().unwrap().0, "\nAssignment history: {:#?}", &tracer.assign_history)
                },
                _ => panic!()
            }
        }
    }
}
