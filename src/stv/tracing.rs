use super::*;

/// Visitor-like pattern.
#[allow(unused_variables)]
pub trait StvTracer<A> {
    fn elem_t(&mut self, a: &A, b: &A, s: &Rat, profile_after: &Profile<A>) {}
    fn strike_out(&mut self, a: &A, profile_after: &Profile<A>) {}

    fn t_toall(&mut self, a: &A, s: &Rat, profile_after: &Profile<A>) {}

    fn electing(&mut self, alts: &HashSet<A>, profile_after: &Profile<A>) {}
    fn eliminating(&mut self, alt: &A, profile_after: &Profile<A>) {}

    fn tie_breaking(&mut self, between: &HashSet<A>, chosen: &A) {}
    fn stv_1winner_tiebreak(&mut self, between: &HashSet<A>, chosen: &A) {}
}

////////////////////////////////////

pub struct NoTracer;
impl<A> StvTracer<A> for NoTracer {}

////////////////////////////////////

pub enum StvAction<A> {
    Elected(HashSet<A>),
    Eliminated(A),
}

pub struct StvLog<A> {
    pub action: StvAction<A>,
    pub profile_afterwards: Profile<A>,
}

pub struct LoggingTracer<A> {
    traces: Vec<StvLog<A>>,
}

impl<A: Clone> StvTracer<A> for LoggingTracer<A> {
    fn electing(&mut self, alts: &HashSet<A>, profile_after: &Profile<A>) {
        self.traces.push(StvLog {
            action: StvAction::Elected(alts.to_owned()),
            profile_afterwards: profile_after.to_owned(),
        });
    }

    fn eliminating(&mut self, alt: &A, profile_after: &Profile<A>) {
        self.traces.push(StvLog {
            action: StvAction::Eliminated(alt.to_owned()),
            profile_afterwards: profile_after.to_owned(),
        });
    }
}

/////////////////////////////////

// pub struct PrintlnTracer<A> {

// }
