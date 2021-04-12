#![allow(dead_code)]
use core::panic;
pub use num_rational::BigRational as Rat; // you could use just `Rational` instead I suppose, it might be marginally faster but might overflow.
use num_traits::{One, ToPrimitive, Zero};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fmt::{Debug, Display},
    hash::Hash,
    write,
};

use self::tracing::{NoTracer, StvTracer};

pub mod tracing;

// from https://stackoverflow.com/questions/28392008/more-concise-hashmap-initialization
macro_rules! hashset {
    ($( $key: expr),*) => {{
            let mut map = ::std::collections::HashSet::new();
            $( map.insert($key); )*
            map
    }}
}

#[derive(Clone)]
pub struct Ballot<A> {
    /// some value between 0 and 1
    pub weight: Rat,
    /// first preference first. this is a "linear order".
    pub preferences: Vec<A>,
}

pub enum CheckBallotResult<A> {
    Ok {
        ballot: Ballot<A>,
        soft_dups: HashSet<A>,
    },
    /// We have something of the form:
    /// `problem` > `other` > `problem`.
    UnresolvableDuplicate {
        problem: A,
    },
    Empty,
}

impl<A: Eq + Hash + Clone> Ballot<A> {
    pub fn from_iter(weight: Rat, slice: impl Iterator<Item = A>) -> CheckBallotResult<A> {
        let mut result = Vec::new();
        let mut soft_dups = HashSet::new();

        let mut dedup = HashSet::new();

        let mut current: Option<A> = None;
        for a in slice {
            if !dedup.insert(a.clone()) {
                let current = current.unwrap(); // safety: have dup ==> at least one elem already scanned ==> have something in current.
                if current == a {
                    // that's okay, just log and continue.
                    soft_dups.insert(a.clone());
                } else {
                    return CheckBallotResult::UnresolvableDuplicate { problem: a };
                }
            } else {
                result.push(a.clone());
            }

            current = Some(a);
        }

        if result.is_empty() {
            return CheckBallotResult::Empty;
        }

        CheckBallotResult::Ok {
            ballot: Ballot {
                weight,
                preferences: result,
            },
            soft_dups,
        }
    }
}

impl<A> Debug for Ballot<A>
where
    A: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(weight) = self.weight.to_f64() {
            write!(f, "{}*[", weight)?;
            let strings: Vec<_> = self
                .preferences
                .iter()
                .map(|p| format!("{:?}", p))
                .collect();
            f.write_str(strings.join(" > ").as_str())?;
            write!(f, "]")?;
        } else {
            write!(f, "(failed to represent ballot)")?;
        }
        Ok(())
    }
}

impl<A: Display> Display for Ballot<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = self
            .preferences
            .iter()
            .map(|p| format!("{}", p))
            .collect::<Vec<_>>();
        f.write_str(x.join(" > ").as_str())
    }
}

impl<A> Ballot<A>
where
    A: PartialEq + Clone + Eq + Hash,
{
    // /// Removes all candidates in `a` in the ballot.
    // pub fn strike_out(&self, a: &HashSet<A>) -> Self {
    //     Self {
    //         weight: self.weight.clone(),
    //         preferences: self
    //             .preferences
    //             .iter()
    //             .filter(|&aa| !a.contains(aa))
    //             .cloned()
    //             .collect(),
    //     }
    // }

    /// Removes all candidates in `a` in the ballot.
    pub fn strike_out_single(&self, a: &A) -> Self {
        Self {
            weight: self.weight.clone(),
            preferences: self.preferences.iter()
                .filter(|&aa| a != aa)
                .cloned()
                .collect()
        }
    }
}

/// (e, r, d) triple
#[derive(Debug, Clone)]
pub struct Result<A> {
    /// Elected
    pub e: HashSet<A>,
    /// Rejected
    pub r: HashSet<A>,
    /// Deferred
    pub d: HashSet<A>,
}

/// A set of ballots. Equivalent to one "election".
#[derive(Clone)]
pub struct Profile<A> {
    pub alts: HashSet<A>,
    pub ballots: Vec<Ballot<A>>,
}

impl<A> Debug for Profile<A>
where
    A: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Profile {{")?;
        // alts, all inline
        f.write_str(
            self.alts
                .iter()
                .map(|p| format!("{:?}", p))
                .collect::<Vec<_>>()
                .join(", ")
                .as_str(),
        )?;
        if self.ballots.is_empty() {
            write!(f, "}} []")?;
        } else {
            writeln!(f, "}} [")?;
            // ballots, one per line
            f.write_str(
                self.ballots
                    .iter()
                    .fold(String::new(), |acc, p| {
                        acc + format!("  {:?},\n", p).as_str()
                    })
                    .as_str(),
            )?;
            write!(f, "]")?;
        }
        Ok(())
    }
}

impl<A: Display + Debug + Clone + Eq + Hash> Display for Profile<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.alts.is_empty() {
            write!(f, "{{ No alternatives! }}")?;
        } else {
            // let maxlen = self.alts.iter()
            //     .map(|alt| format!("{}", alt).len())
            //     .max().unwrap(); // unwrap is safe because we checked for is_empty()
            write!(f, "{{")?;

            let scores = self.scores();
            let mut tuples: Vec<(&A, &Rat)> = scores.iter().collect();
            tuples.sort_by(|a, b| b.1.cmp(a.1));
            for score in tuples {
                // print alt name with padding
                // write!(f, "{:width$}", score.0, width = maxlen)?;
                write!(f, "{} -> {}, ", score.0, score.1)?;
            }
            write!(f, "}}")?;
        }
        Ok(())
    }
}

impl<A> Profile<A>
where
    A: Clone + Eq + Hash + Debug,
{
    /// total amount of first preferences votes for a given candidate.
    pub fn score(&self, alt: &A) -> Rat {
        let score = self
            .ballots
            .iter()
            .filter(|ballot| !ballot.preferences.is_empty() && &ballot.preferences[0] == alt)
            .fold(Rat::zero(), |acc, ballot| acc + &ballot.weight);
        // println!("Score of {:?} in {:?}: {}", alt, self, score.to_f32().unwrap());
        score
    }

    /// Sums all weights together.
    ///
    /// This is different from just counting the amount of ballots, because:
    /// - Ballots may have a weight of 2
    /// - Ballots may have a weight of 0.5
    pub fn weight_sum(&self) -> Rat {
        self.ballots.iter().map(|b| &b.weight).sum()
    }

    /// Elementary STV vote transfer function.
    ///
    /// Transfers from `a` to `b`, exactly `s` much. Where `s` is a number between 0 and 1.
    /// - `s = 0`: This function is a no-op, nothing is transferred.
    /// - `s = 1`: Transfer everything, leave no residual.
    pub fn elem_t<T: StvTracer<A>>(&self, a: &A, b: &A, s: &Rat, tracer: &mut T) -> Self {
        if *s < Rat::zero() || *s > Rat::one() {
            panic!(
                "Single Transferable Vote elem_t got s out of bounds! s = {}",
                s
            );
        }
        if Rat::is_zero(s) {
            tracer.elem_t(a, b, s, self);
            return self.clone();
        }

        // println!("elem_t {:?} {:?} {}", a, b, s);

        let mut ret = Vec::<Ballot<A>>::new();
        for ballot in self
            .ballots
            .iter()
            .filter(|&b| !Rat::is_zero(&b.weight) && !b.preferences.is_empty())
        {
            if ballot.preferences.len() >= 2 && &ballot.preferences[0] == a && &ballot.preferences[1] == b {
                // bingo, ballots where first preference is `a` and second `b` is where we transfer stuff!
                // split ballot up into residual and transferred part.

                if !Rat::is_one(s) {
                    // if s == 1, then we would just create empty ballots here. not technically wrong, but
                    // might as well skip them. small optimization.
                    let residual = Ballot {
                        weight: &ballot.weight * (Rat::one() - s),
                        preferences: ballot.preferences.clone(),
                    };
                    ret.push(residual);
                }

                let transfer = Ballot {
                    weight: &ballot.weight * s,
                    preferences: ballot.preferences[1..].into(), // remove `a` from transferred ballots.
                };
                ret.push(transfer);
            } else {
                // otherwise do nothing.
                ret.push(ballot.clone());
            }
        }

        let profile = Profile {
            alts: self.alts.clone(),
            ballots: ret,
        };

        tracer.elem_t(a, b, s, &profile);

        profile
    }

    /// Transfer votes from `a` to all other alternatives.
    pub fn t_to_all<T: StvTracer<A>>(&self, a: &A, s: &Rat, tracer: &mut T) -> Self {
        let mut profile = self.clone();
        // I think (for single seat at least), it's completely irrelavant in which order we do this?
        for b in self.alts.iter().filter(|&alt| alt != a) {
            profile = profile.elem_t(a, b, s, tracer);
        }

        tracer.t_toall(a, s, &profile);

        profile
    }

    // /// `limit_to` basically.
    // /// Removes candidate `a` from all ballots, and from `alts`
    // pub fn strike_out(&self, eliminated: &HashSet<A>) -> Self {
    //     Self {
    //         alts: self.alts.difference(eliminated).cloned().collect(),
    //         ballots: self
    //             .ballots
    //             .iter()
    //             // .filter(|&b| b.)
    //             .map(|b| b.strike_out(eliminated))
    //             .collect(),
    //     }
    // }

    /// `consume` basically.
    /// Removes candidate `a` from all ballots, and from `alts`
    pub fn strike_out_single<T: StvTracer<A>>(&self, eliminated: &A, tracer: &mut T) -> Self {
        // let profile = self.strike_out(&[eliminated.clone()].iter().cloned().collect());
        let profile = Self {
            alts: self.alts.iter()
                .filter(|&a| a != eliminated)
                .cloned()
                .collect(),
            ballots: self.ballots.iter()
                .filter(|&b| !b.preferences.is_empty() && &b.preferences[0] != eliminated)
                .map(|b| b.strike_out_single(eliminated))
                .collect()
        };
        tracer.strike_out(eliminated, &profile);
        profile
    }

    /// Calculates the score of each alternative. E.g.:
    /// - "Wolf" -> 3.5
    /// - "Penguin" -> 1
    /// - etc...
    pub fn scores(&self) -> HashMap<A, Rat> {
        self.alts
            .iter()
            .map(|alt| (alt.to_owned(), self.score(&alt)))
            .collect()
    }

    /// Expects quota in absolute form.
    #[allow(non_snake_case)]
    pub fn vanilla_T<T: StvTracer<A>>(&self, q: &Rat, result: &Result<A>, tracer: &mut T) -> Self {
        let mut profile = self.clone();
        for e in &result.e {
            let score = self.score(e);
            // transfer surplus
            profile = profile.t_to_all(e, &((&score - q) / &score), tracer);
            profile = profile.strike_out_single(e, tracer);
        }
        if !result.e.is_empty() {
            tracer.electing(&result.e, &profile);
        }

        for r in &result.r {
            // transfer everything
            profile = profile.t_to_all(r, &Rat::one(), tracer);
            profile = profile.strike_out_single(r, tracer);
            tracer.eliminating(&r, &profile);
        }

        profile
    }

    pub fn cmp(&self, a: &A, b: &A) -> Ordering {
        let a = self.score(a);
        let b = self.score(b);
        a.cmp(&b)
    }

    /// `it = (threshold q || drop 1 (worst))`.
    /// Guarantees that `d` decreases by at least 1, unless we reached a fixed point.
    pub fn elect_or_reject<T: StvTracer<A>>(&self, q: &Rat, tracer: &mut T) -> Result<A> {
        // get everyone who crossed quota
        let elected: HashSet<_> = self
            .alts
            .iter()
            .filter(|&alt| &self.score(alt) >= q)
            .cloned()
            .collect();
        if elected.is_empty() {
            // otherwise, eliminate the worst

            // Find the score minimum.
            let mut min: Option<Rat> = None;
            for alt in self.alts.iter() {
                let score = self.score(alt);
                if let Some(min2) = &min {
                    if &score < min2 {
                        min = Some(score);
                    }
                } else {
                    min = Some(score);
                }
            }

            // it is possible that we may not have a minimum (e.g. alts empty).
            if let Some(min) = min {
                let worst_set = self
                    .alts
                    .iter()
                    .filter(|alt| self.score(alt) == min)
                    .cloned()
                    .collect::<HashSet<_>>();
                let worst = worst_set.iter().find(|_| true).unwrap(); // if we have a minimum, worst_set can not be empty. Thus the unwrap is safe.
                if worst_set.len() > 1 {
                    tracer.reject_tie_break(&worst_set, worst, &min);
                }
                let r = hashset![worst.clone()];
                let d = self.alts.difference(&r).cloned().collect(); // d = A \ {worst}
                Result {
                    e: HashSet::new(), // no elected, empty
                    r,                 // just one element.
                    d,
                }
            } else {
                Result {
                    e: HashSet::new(),    // empty
                    r: HashSet::new(),    // empty
                    d: self.alts.clone(), // we clone it for correctness, but this will always be empty, otherwise we wouldn't be here.
                }
            }
        } else {
            // elect them all
            let d = self.alts.difference(&elected).cloned().collect(); // d = A \ e
            Result {
                e: elected,
                r: HashSet::new(), // no rejected, empty.
                d,
            }
        }
    }

    /// performs a single iteration of vSTV.
    pub fn vanilla_stv_step<T: StvTracer<A>>(&self, q: &Rat, tracer: &mut T) -> (Result<A>, Self) {
        let result = self.elect_or_reject(q, tracer);
        let profile = self.vanilla_T(q, &result, tracer);
        (result, profile)
    }

    pub fn vanilla_stv<T: StvTracer<A>>(&self, seats: usize, q: &Rat, tracer: &mut T) -> Result<A> {
        if self.alts.len() <= seats {
            // if we only have `seats` candidates left, just elect everyone, even if they don't cross quota.
            // case of one bf4map only: means only one map had been nominated.
            tracer.electing(&self.alts, self); // no profile change.
            return Result {
                e: self.alts.clone(),
                r: HashSet::new(),
                d: HashSet::new(),
            };
        }

        // so now we have at least `seats + 1` alternatives left.

        let (result, profile) = self.vanilla_stv_step(q, tracer);
        assert_eq!(profile.alts, result.d);
        if result.d.is_empty() || result.e.len() >= seats {
            // if we either filled all seats, or exhausted candidates, we're done.
            result
        } else {
            // otherwise, recursive call to fill remaining open seats, and then append our thus-far elected candidates.
            let inner = profile.vanilla_stv(seats - result.e.len(), q, tracer);
            Result {
                e: result.e.union(&inner.e).cloned().collect(),
                r: result.r.union(&inner.r).cloned().collect(),
                d: inner.d, // d gets smaller each iterator. Ideally we return d = {}.
            }

            // result.union3(&inner)
        }
    }

    /// Stops as soon as the first candidate is elected.
    ///
    /// Works with droop quota with one seat.
    ///
    /// # Returns
    /// - `Some(winner)` if there was a winner.
    /// - `None` if winner couldn't be determined.
    pub fn vanilla_stv_1<T: StvTracer<A>>(&self, tracer: &mut T) -> Option<A> {
        // let q = self.ballots.len() / 2 + 1; // Droop quota for one seat.
        // let q = Rat::from_integer(BigInt::from_usize(q).unwrap());
        let q = self.weight_sum() / (Rat::one() + Rat::one());
        let q = q.floor();
        let q = q + Rat::one();
        let result = self.vanilla_stv(1, &q, tracer);

        let winner = result.e.iter().find(|_| true).cloned();
        if result.e.len() > 1 {
            // if used with droop quota, this branch is impossible.
            let winner = winner.clone().unwrap(); // len > 1, means find definitely finds something, so unwrap is safe here.
            tracer.stv_1winner_tiebreak(&result.e, &winner);
        }
        winner
    }

    /// Stops as soon as the first candidate is elected.
    ///
    /// Works with droop quota with one seat.
    ///
    /// # Returns
    /// - `Some((winner, X))` if there was a winner.
    ///    - The X is the runner-up, with similar Some(runnerup) or None.
    /// - `None` if winner couldn't be determined.
    pub fn vanilla_stv_1_with_runnerup<T: StvTracer<A>>(
        &self,
        tracer: &mut T,
        tracer_runnerup: &mut T,
    ) -> Option<(A, Option<A>)> {
        if let Some(winner) = self.vanilla_stv_1(tracer) {
            let runnerup = self
                .strike_out_single(&winner, &mut NoTracer)
                .vanilla_stv_1(tracer_runnerup);
            Some((winner, runnerup))
        } else {
            None
        }
    }
}

#[cfg(test)]
pub mod test {
    use std::fmt::{Debug, Display};
    use std::hash::Hash;

    use super::tracing::*;
    use super::{Ballot, Profile};
    use num_rational::BigRational as Rat; // you could use just `Rational` instead I suppose, it might be marginally faster but might overflow.
    use num_traits::One;

    macro_rules! ballot {
        [1, $($pref: expr),*] => {{
            Ballot {
                weight: Rat::one(),
                preferences: vec![
                    $($pref),*
                ],
            }
        }};
        [2, $($pref: expr),*] => {{
            Ballot {
                weight: Rat::one() + Rat::one(),
                preferences: vec![
                    $($pref),*
                ],
            }
        }};
        [0.5, $($pref: expr),*] => {{
            Ballot {
                weight: Rat::one() / (Rat::one() + Rat::one()),
                preferences: vec![
                    $($pref),*
                ],
            }
        }};
    }

    fn print_trace<A>(start: &Profile<A>, tracer: ElectElimTiebreakTracer<A>)
    where
        // T: StvTracer<A>,
        // T::Trace: IntoIterator<Item = StvAction<A>>,
        A: Display + Debug + Clone + Eq + Hash + 'static,
    {
        println!("Starting with {}", start);
        for action in tracer.trace {
            if let Some(p) = action.get_profile_after() {
                println!("{} ==> {}", &action, p); // Change to "{} ==> {:?}" if you want all ballots listed, not just the scores.
            } else {
                println!("{}", &action);
            }
        }
    }

    fn print_trace2<A>(start: &Profile<A>, tracer: &DetailedTracer<A>)
    where
        // T: StvTracer<A>,
        // T::Trace: IntoIterator<Item = StvAction<A>>,
        A: Display + Debug + Clone + Eq + Hash + 'static,
    {
        println!("Starting with {}", start);
        for action in &tracer.trace {
            if let Some(p) = action.get_profile_after() {
                println!("{} ==> {}", &action, p); // Change to "{} ==> {:?}" if you want all ballots listed, not just the scores.
            } else {
                println!("{}", &action);
            }
        }
    }

    #[test]
    fn stv() {
        let profile = Profile {
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

        let mut tracer = ElectElimTiebreakTracer::new();

        let winner = profile.vanilla_stv_1(&mut tracer).unwrap();

        print_trace(&profile, tracer);

        assert_eq!("Wolf", winner);
        // assert!(false);
    }

    #[test]
    fn ballotweight_2() {
        let profile = Profile {
            alts: hashset!["Wolf", "Fox", "Eagle", "Penguin"],
            ballots: vec![
                ballot![2, "Eagle"],
                ballot![2, "Eagle"],
                ballot![2, "Eagle"],
                ballot![0.5, "Wolf", "Fox", "Eagle"],
                ballot![2, "Fox", "Wolf", "Eagle"],
                ballot![2, "Wolf", "Fox", "Eagle"],
                ballot![2, "Wolf", "Fox"],
            ],
        };

        let mut tracer = ElectElimTiebreakTracer::new();

        let winner = profile.vanilla_stv_1(&mut tracer).unwrap();

        print_trace(&profile, tracer);

        assert_eq!("Wolf", winner);
        // assert!(false);
    }

    /// When ballots aren't full (all candidates on each ballot), then transfering votes has a
    /// problem. Look at the output and you'll see
    #[test]
    #[ignore]
    fn unfull_ballots_broken() {
        let profile = Profile {
            alts: hashset!["Wolf", "Fox"],
            ballots: vec![
                ballot![1, "Fox"],
                ballot![1, "Fox", "Wolf"],
                ballot![1, "Fox", "Wolf"],
                ballot![1, "Wolf", "Fox"],
            ],
        };
        println!("Starting with: {:?}", profile);

        let mut tracer = DetailedTracer::new();

        let two = Rat::one() + Rat::one();

        let result = profile.vanilla_stv(1, &two, &mut tracer);
        println!("Result: {:?}", result);

        print_trace2(&profile, &tracer);
        let profile2 = tracer.trace[0].get_profile_after().unwrap();
        println!("After first step: {:?}", profile2);
        assert_eq!(two, profile2.score(&"Fox"));
        assert_eq!(two, profile2.score(&"Wolf"));
    }

    #[test]
    fn full_ballots_elemt_exact_shave_off() {
        let profile = Profile {
            alts: hashset!["Wolf", "Fox", "No"],
            ballots: vec![
                ballot![1, "Fox", "No"],
                ballot![1, "Fox", "Wolf"],
                ballot![1, "Fox", "Wolf"],
                ballot![1, "Wolf", "Fox"],
            ],
        };
        println!("Starting with: {:?}", profile);

        let mut tracer = DetailedTracer::new();

        let two = Rat::one() + Rat::one();

        let result = profile.vanilla_stv(1, &two, &mut tracer);
        println!("Result: {:?}", result);

        print_trace2(&profile, &tracer);
        let profile2 = tracer.trace[1].get_profile_after().unwrap();
        println!("After second step: {:?}", profile2); // after we've done Fox -> No, Fox -> Wolf (both at 1/3).
        assert_eq!(two, profile2.score(&"Fox"));
    }
}
