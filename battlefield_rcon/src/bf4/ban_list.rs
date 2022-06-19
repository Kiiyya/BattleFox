use std::time::Duration;

use ascii::IntoAsciiString;

use super::{RconEncoding, Eaid};


#[derive(Clone, Debug)]
pub enum Ban {
    Name(String),
    Ip(String),
    Guid(Eaid),
}

#[derive(Clone, Debug)]
pub enum BanTimeout {
    Permanent,
    Rounds(usize),
    Time(Duration),
}

// impl RconEncoding for BanTimeout {
//     fn rcon_encode(&self) -> ascii::AsciiString {
//         match self {
//             BanTimeout::Permanent => "perm".to_string(),
//             BanTimeout::Rounds(rounds) => format!("rounds {rounds}"),
//             BanTimeout::Time(dur) => format!("seconds {}", dur.as_secs()),
//         }.into_ascii_string().unwrap()
//     }
// }