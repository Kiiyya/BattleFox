use super::*;

/// Visitor-like pattern.
#[allow(unused_variables)]
pub trait StvTracer<A> {
    fn elem_t(&mut self, a: &A, b: &A, s: &Rat, profile_after: &Profile<A>) {}
    fn strike_out(&mut self, a: &A, profile_after: &Profile<A>) {}

    fn t_toall(&mut self, a: &A, s: &Rat, profile_after: &Profile<A>) {}

    fn electing(&mut self, alts: &HashSet<A>, profile_after: &Profile<A>) {}
    fn eliminating(&mut self, alt: &A, profile_after: &Profile<A>) {}

    /// When there's two or more alts tied for last place.
    fn reject_tie_break(&mut self, between: &HashSet<A>, chosen: &A, score: &Rat) {}
    fn stv_1winner_tiebreak(&mut self, between: &HashSet<A>, chosen: &A) {}
}

////////////////////////////////////

pub struct NoTracer;
impl<A> StvTracer<A> for NoTracer {}

////////////////////////////////////

#[derive(Debug)]
pub enum StvAction<A> {
    ElemT {
        a: A,
        b: A,
        s: Rat,
        profile_afterwards: Profile<A>,
    },
    StrikeOut {
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

impl<A> StvAction<A> {
    pub fn get_profile_after(&self) -> Option<&Profile<A>> {
        match self {
            StvAction::ToAll {
                from: _,
                howmuch: _,
                profile_afterwards,
            } => Some(&profile_afterwards),
            StvAction::StrikeOut {
                alt: _,
                profile_afterwards,
            } => Some(&profile_afterwards),
            StvAction::RejectTiebreak {
                tied: _,
                chosen: _,
                score: _,
            } => None,
            StvAction::Elected {
                elected: _,
                profile_afterwards,
            } => Some(&profile_afterwards),
            StvAction::Eliminated {
                alt: _,
                profile_afterwards,
            } => Some(&profile_afterwards),
            StvAction::ElemT {
                a: _,
                b: _,
                s: _,
                profile_afterwards,
            } => Some(&profile_afterwards),
            StvAction::Stv1WinnerTiebreak { tied: _, chosen: _ } => None,
        }
    }
}

impl<A: Display + Debug> Display for StvAction<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StvAction::ToAll {
                from,
                howmuch,
                profile_afterwards: _,
            } => write!(f, "ToAll({}, {}) ==>", from, howmuch),
            StvAction::StrikeOut {
                alt,
                profile_afterwards: _,
            } => write!(f, "StrikeOut({})", alt),
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
                write!(f, "Stv1WinnerTieBreak({:?}, {}", tied, chosen)
            }
        }
    }
}

/// Only logs the following events:
/// - Elected
/// - Eliminated
/// - Reject Tiebreak
#[derive(Debug)]
pub struct ElectElimTiebreakTracer<A> {
    pub traces: Vec<StvAction<A>>,
}

impl<A> ElectElimTiebreakTracer<A> {
    pub fn new() -> Self {
        Self { traces: Vec::new() }
    }
}

impl<A: Clone> StvTracer<A> for ElectElimTiebreakTracer<A> {
    fn electing(&mut self, alts: &HashSet<A>, profile_after: &Profile<A>) {
        self.traces.push(StvAction::Elected {
            elected: alts.to_owned(),
            profile_afterwards: profile_after.to_owned(),
        })
    }

    fn eliminating(&mut self, alt: &A, profile_after: &Profile<A>) {
        self.traces.push(StvAction::Eliminated {
            alt: alt.to_owned(),
            profile_afterwards: profile_after.to_owned(),
        })
    }

    fn reject_tie_break(&mut self, between: &HashSet<A>, chosen: &A, score: &Rat) {
        self.traces.push(StvAction::RejectTiebreak {
            tied: between.to_owned(),
            chosen: chosen.to_owned(),
            score: score.to_owned(),
        })
    }
}

/// Logs all events, even the very low-level ones.
#[derive(Debug)]
pub struct DetailedTracer<A> {
    pub traces: Vec<StvAction<A>>,
}

impl<A> DetailedTracer<A> {
    pub fn new() -> Self {
        Self { traces: Vec::new() }
    }
}

impl<A: Clone> StvTracer<A> for DetailedTracer<A> {
    fn elem_t(&mut self, a: &A, b: &A, s: &Rat, profile_after: &Profile<A>) {
        self.traces.push(StvAction::ElemT {
            a: a.to_owned(),
            b: b.to_owned(),
            s: s.to_owned(),
            profile_afterwards: profile_after.to_owned(),
        })
    }

    fn electing(&mut self, alts: &HashSet<A>, profile_after: &Profile<A>) {
        self.traces.push(StvAction::Elected {
            elected: alts.to_owned(),
            profile_afterwards: profile_after.to_owned(),
        })
    }

    fn eliminating(&mut self, alt: &A, profile_after: &Profile<A>) {
        self.traces.push(StvAction::Eliminated {
            alt: alt.to_owned(),
            profile_afterwards: profile_after.to_owned(),
        })
    }

    fn t_toall(&mut self, a: &A, s: &Rat, profile_after: &Profile<A>) {
        self.traces.push(StvAction::ToAll {
            from: a.to_owned(),
            howmuch: s.to_owned(),
            profile_afterwards: profile_after.to_owned(),
        })
    }

    fn strike_out(&mut self, a: &A, profile_after: &Profile<A>) {
        self.traces.push(StvAction::StrikeOut {
            alt: a.to_owned(),
            profile_afterwards: profile_after.to_owned(),
        })
    }

    fn reject_tie_break(&mut self, between: &HashSet<A>, chosen: &A, score: &Rat) {
        self.traces.push(StvAction::RejectTiebreak {
            tied: between.to_owned(),
            chosen: chosen.to_owned(),
            score: score.to_owned(),
        })
    }

    fn stv_1winner_tiebreak(&mut self, between: &HashSet<A>, chosen: &A) {
        self.traces.push(StvAction::Stv1WinnerTiebreak {
            tied: between.to_owned(),
            chosen: chosen.to_owned(),
        })
    }
}

/////////////////////////////////

// pub struct PrintlnTracer<A> {

// }
