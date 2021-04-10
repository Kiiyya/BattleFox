//! Some helpers for dealing with e.g. plurals, "is", "are", etc.

pub enum Time {
    /// "is", "are".
    Present,

    /// Present Perfect Progressive
    ///
    /// "has been", "have been".
    PrPrfPrg,
}

pub enum Num {
    /// Singular.
    Sg,

    /// Plural.
    Pl,
}

pub enum Verb {
    // "is", "are", "have been", etc.
    Be,
}

impl From<usize> for Num {
    fn from(n: usize) -> Self {
        match n {
            0 => Num::Pl,
            1 => Num::Sg,
            _ => Num::Pl,
        }
    }
}

pub trait HumanLang {
    fn time() -> Time;
    fn numerus() -> Num;
}

pub struct Wordz {
    pub time: Time,
    pub numerus: Num,
}

pub fn be(time: Time, num: impl Into<Num>) -> &'static str {
    let num = num.into();
    match time {
        Time::Present => match num {
            Num::Sg => "is",
            Num::Pl => "are",
        }
        Time::PrPrfPrg => match num {
            Num::Sg => "has been",
            Num::Pl => "have been",
        }
    }
}

pub fn hlang(verb: Verb, time: Time, num: impl Into<Num>) -> &'static str {
    match verb {
        Verb::Be => be(time, num)
    }
}