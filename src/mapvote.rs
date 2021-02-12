#![allow(unused_variables, unused_imports)]
// use std::collections::HashMap;

// enum Bf4Map {
//     Locker,
//     Metro,
//     PearlMarket,
// }

// struct Ballot {
//     preferences: Vec<Bf4Map>,
// }

// struct MapVote {
//     votes: HashMap<String, Ballot>,
// }

// impl MapVote {
//     fn calculate_result(&self) {}
// }

/*
So, you want to have a state-only thing which only manages state, and handles the database. aka the model.
For mapvote we don't need much, but yeah.


Maybe we'll have just one giant onEvent, which has a giant `match`, and then calls mapvote etc with a fitting argument already.
A central router, a bit like asp.net. If you ever want more complexity or extensibility, then you can add some fancy fluid builder
methods or something.
*/
