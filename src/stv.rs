#![allow(dead_code)]
use core::panic;
use num_bigint::BigInt;
use num_rational::BigRational as Rat; // you could use just `Rational` instead I suppose, it might be marginally faster but might overflow.
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use std::{cmp::Ordering, collections::HashSet, fmt::Debug, hash::Hash, write};

#[derive(Clone)]
pub struct Ballot<A> {
    /// some value between 0 and 1
    pub weight: Rat,
    /// first preference first. this is a "linear order".
    pub preferences: Vec<A>,
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

impl<A> Ballot<A>
where
    A: PartialEq + Clone + Eq + Hash,
{
    /// Removes all candidates in `a` in the ballot.
    pub fn strike_out(&self, a: &HashSet<A>) -> Self {
        Self {
            weight: self.weight.clone(),
            preferences: self
                .preferences
                .iter()
                .filter(|&aa| !a.contains(aa))
                .cloned()
                .collect(),
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

// impl <A> Display for Profile<A>
// {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

//     }
// }

impl<A> Profile<A>
where
    A: PartialEq + Clone + Eq + Hash + Debug,
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

    /// Elementary STV vote transfer function.
    ///
    /// Transfers from `a` to `b`, exactly `s` much. Where `s` is a number between 0 and 1.
    /// - `s = 0`: This function is a no-op, nothing is transferred.
    /// - `s = 1`: Transfer everything, leave no residual.
    pub fn elem_t(&self, a: &A, b: &A, s: &Rat) -> Self {
        if *s < Rat::zero() || *s > Rat::one() {
            panic!(
                "Single Transferable Vote elem_t got s out of bounds! s = {}",
                s
            );
        }
        if Rat::is_zero(s) {
            return self.clone();
        }

        let mut ret = Vec::<Ballot<A>>::new();
        for ballot in self
            .ballots
            .iter()
            .filter(|&b| !Rat::is_zero(&b.weight) && !b.preferences.is_empty())
        {
            if ballot.preferences[0] == *a {
                if ballot.preferences.len() >= 2 {
                    if ballot.preferences[1] == *b {
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
                } else {
                    // we have nobody to transfer to, so we yeet this ballot entirely.
                    continue;
                }
            } else {
                // otherwise do nothing.
                ret.push(ballot.clone());
            }
        }

        Self {
            alts: self.alts.clone(),
            ballots: ret,
        }
    }

    /// Transfer votes from `a` to all other alternatives.
    pub fn t_to_all(&self, a: &A, s: &Rat) -> Self {
        let mut profile = self.clone();
        // I think (for single seat at least), it's completely irrelavant in which order we do this?
        for b in self.alts.iter().filter(|&alt| alt != a) {
            profile = profile.elem_t(a, b, s);
        }

        profile
    }

    /// `limit_to` basically.
    /// Removes candidate `a` from all ballots, and from `alts`
    pub fn strike_out(&self, eliminated: &HashSet<A>) -> Self {
        Self {
            alts: self.alts.difference(eliminated).cloned().collect(),
            ballots: self
                .ballots
                .iter()
                .map(|b| b.strike_out(eliminated))
                .collect(),
        }
    }

    /// `limit_to` basically.
    /// Removes candidate `a` from all ballots, and from `alts`
    pub fn strike_out_single(&self, eliminated: &A) -> Self {
        self.strike_out(&[eliminated.clone()].iter().cloned().collect())
    }

    /// Expects quota in absolute form.
    #[allow(non_snake_case)]
    pub fn vanilla_T(&self, q: &Rat, result: &Result<A>) -> Self {
        let mut profile = self.clone();
        for e in &result.e {
            let score = self.score(e);
            // transfer surplus
            profile = profile.t_to_all(e, &(&score - q));
            profile = profile.strike_out_single(e);
        }

        for r in &result.r {
            // transfer everything
            profile = profile.t_to_all(r, &Rat::one());
            profile = profile.strike_out_single(r);
        }

        profile
    }

    pub fn cmp(&self, a: &A, b: &A) -> Ordering {
        let a = self.score(a);
        let b = self.score(b);
        a.cmp(&b)
    }

    /// `it = (threshold q || drop 1 (worst))`.
    /// Guarantees at `d` decreases by at least 1, unless we reached a fixed point.
    pub fn one_iteration(&self, q: &Rat) -> Result<A> {
        // get everyone who crossed quota
        let elected: HashSet<_> = self
            .alts
            .iter()
            .filter(|&alt| &self.score(alt) >= q)
            .cloned()
            .collect();
        if !elected.is_empty() {
            // elect them all
            let d = self.alts.difference(&elected).cloned().collect(); // d = A \ e
            Result {
                e: elected,
                r: HashSet::new(), // no rejected, empty.
                d,
            }
        } else {
            // otherwise, eliminate the worst
            // TODO: Maybe implement parallel universe tie breaking? (min_by just selects the first minimum found)
            if let Some(worst) = self.alts.iter().min_by(|&alt1, &alt2| self.cmp(alt1, alt2)) {
                let r = [worst.clone()].iter().cloned().collect();
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
        }
    }

    /// performs a single iteration of vSTV.
    pub fn vanilla_stv_step(&self, q: &Rat) -> (Result<A>, Self) {
        let result = self.one_iteration(q);
        let profile = self.vanilla_T(q, &result);
        (result, profile)
    }

    pub fn vanilla_stv(&self, seats: usize, q: &Rat) -> Result<A> {
        if self.alts.len() <= seats {
            // if we only have `seats` candidates left, just elect everyone, even if they don't cross quota.
            // case of one bf4map only: means only one map had been nominated.
            return Result {
                e: self.alts.clone(),
                r: HashSet::new(),
                d: HashSet::new(),
            };
        }

        // so now we have at least `seats + 1` alternatives left.

        let (result, profile) = self.vanilla_stv_step(q);
        assert_eq!(profile.alts, result.d);
        if result.d.is_empty() || result.e.len() >= seats {
            // if we either filled all seats, or exhausted candidates, we're done.
            result
        } else {
            // otherwise, recurive call to fill remaining open seats, and then append our thus-far elected candidates.
            let inner = profile.vanilla_stv(seats - result.e.len(), q);
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
    /// Works with `q = 100%` (i.e. Hare Quota with one seat), meaning that a candidate eventually needs to get 100% of the votes.
    ///
    /// # Returns
    /// - `Some(winner)` if there was a winner.
    /// - `None` if winner couldn't be determined.
    pub fn vanilla_stv_1(&self) -> Option<A> {
        let q = &Rat::from_integer(BigInt::from_usize(self.ballots.len()).unwrap());
        let result = self.vanilla_stv(1, q);

        result.e.iter().find(|_| true).cloned()
    }
}

#[cfg(test)]
pub mod test {
    use super::{Ballot, Profile};
    use num_rational::BigRational as Rat; // you could use just `Rational` instead I suppose, it might be marginally faster but might overflow.
    use num_traits::One;

    // from https://stackoverflow.com/questions/28392008/more-concise-hashmap-initialization
    macro_rules! hashset {
        ($( $key: expr),*) => {{
             let mut map = ::std::collections::HashSet::new();
             $( map.insert($key); )*
             map
        }}
    }

    macro_rules! ballot {
        [$($pref: expr),*] => {{
            Ballot {
                weight: Rat::one(),
                preferences: vec![
                    $($pref),*
                ],
            }
        }}
    }

    #[test]
    fn stv() {
        let profile = Profile {
            alts: hashset!["Wolf", "Fox", "Eagle", "Penguin"],
            ballots: vec![
                ballot!["Eagle"],
                ballot!["Eagle"],
                ballot!["Eagle"],
                ballot!["Eagle"],
                ballot!["Wolf", "Fox", "Eagle"],
                ballot!["Fox", "Wolf", "Eagle"],
                ballot!["Wolf", "Fox", "Eagle"],
                ballot!["Wolf", "Fox"],
            ],
        };

        let winner = profile.vanilla_stv_1().unwrap();
        assert_eq!("Wolf", winner);
    }
}
