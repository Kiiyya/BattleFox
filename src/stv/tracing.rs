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
    ToAll {
        from: A,
        howmuch: Rat,
        profile_afterwards: Profile<A>,
    },
    StrikeOut {
        alt: A,
        profile_afterwards: Profile<A>,
    },
    RejectTieBreak {
        tied: HashSet<A>,
        chosen: A,
        score: Rat,
    },
    Elected {
        elected: HashSet<A>,
        profile_afterwards: Profile<A>,
    },
    Eliminated {
        alt: A,
        profile_afterwards: Profile<A>,
    }
}

#[derive(Debug)]
pub struct LoggingTracer<A> {
    pub traces: Vec<StvAction<A>>,
}

impl <A> LoggingTracer<A> {
    pub fn new() -> Self {
        Self {
            traces: Vec::new(),
        }
    }
}

impl<A: Clone> StvTracer<A> for LoggingTracer<A> {
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

    // fn t_toall(&mut self, a: &A, s: &Rat, profile_after: &Profile<A>) {
    //     self.traces.push(StvAction::ToAll {
    //         from: a.to_owned(),
    //         howmuch: s.to_owned(),
    //         profile_afterwards: profile_after.to_owned(),
    //     })
    // }

    // fn strike_out(&mut self, a: &A, profile_after: &Profile<A>) {
    //     self.traces.push(StvAction::StrikeOut {
    //         alt: a.to_owned(),
    //         profile_afterwards: profile_after.to_owned(),
    //     })
    // }

    fn reject_tie_break(&mut self, between: &HashSet<A>, chosen: &A, score: &Rat) {
        self.traces.push(StvAction::RejectTieBreak {
            tied: between.to_owned(),
            chosen: chosen.to_owned(),
            score: score.to_owned(),
        })
    }
}

/////////////////////////////////

// pub struct PrintlnTracer<A> {

// }
