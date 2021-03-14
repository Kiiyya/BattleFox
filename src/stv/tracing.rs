use super::*;

/// Visitor-like pattern.
#[allow(unused_variables)]
pub trait StvTracer<A> {
    fn elem_t(&mut self, a: &A, b: &A, s: &Rat, profile_after: &Profile<A>) { }
    fn strike_out(&mut self, a: &A, profile_after: &Profile<A>) { }

    fn t_toall(&mut self, a: &A, s: &Rat, profile_after: &Profile<A>) { }

    fn electing(&mut self, alts: &HashSet<A>, profile_after: &Profile<A>) { }
    fn eliminating(&mut self, alt: &A, profile_after: &Profile<A>) { }
    fn tie_breaking(&mut self, left: &A, right: &A, chosen: &A, profile_after: &Profile<A>) { }
}

////////////////////////////////////

pub struct NoTracer;
impl <A> StvTracer<A> for NoTracer { }

////////////////////////////////////

pub enum StvAction<A> {
    Electing(HashSet<A>),
}

pub struct StvLog<A> {
    pub action: StvAction<A>,
    pub profile_afterwards: Profile<A>,
}

pub struct LoggingTracer<A> {
    traces: Vec<StvLog<A>>,
}

impl <A: Clone> StvTracer<A> for LoggingTracer<A> {
    fn electing(&mut self, alts: &HashSet<A>, profile_after: &Profile<A>) {
        self.traces.push(StvLog {
            action: StvAction::Electing(alts.to_owned()),
            profile_afterwards: profile_after.to_owned(),
        });
    }
}

/////////////////////////////////

// pub struct TextTracer
