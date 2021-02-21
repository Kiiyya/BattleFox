#![allow(unused_variables, unused_imports)]

use super::stv::Ballot;
use battlefield_rcon::bf4::Player;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
enum Bf4Map {
    Locker,
    Metro,
    PearlMarket,
}

#[derive(Debug)]
struct MapVote {
    votes: HashMap<Player, Ballot<Bf4Map>>,
}

impl MapVote {
    /// returns Some(old_ballot) if player had voted before.
    pub fn vote(&mut self, player: Player, ballot: Ballot<Bf4Map>) -> Option<Ballot<Bf4Map>> {
        self.votes.insert(player, ballot)
    }
}

/*
So, you want to have a state-only thing which only manages state, and handles the database. aka the model.
For mapvote we don't need much, but yeah.


Maybe we'll have just one giant onEvent, which has a giant `match`, and then calls mapvote etc with a fitting argument already.
A central router, a bit like asp.net. If you ever want more complexity or extensibility, then you can add some fancy fluid builder
methods or something.
*/
