use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use ascii::{AsciiStr, AsciiString};

use crate::{bf4::RconEncoding, rcon::RconResult};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Map {
    Metro,
    Locker,
    PearlMarket,
    Oman,
    Other(AsciiString),
}

impl Map {
    pub fn pretty(&self) -> &'static str {
        match self {
            Map::Metro => "Metro",
            Map::Locker => "Locker",
            Map::PearlMarket => "Pearl Market",
            Map::Oman => "Gulf of Oman",
            Map::Other(_) => "(Some other map)", // FIXME some day we won't have "Other".
        }
    }

    /// Gets an iterator over all currently known maps. (Excluding `Other(...)`).
    pub fn all() -> std::slice::Iter<'static, Map> {
        static ALL: [Map; 4] = [Map::Metro, Map::Locker, Map::PearlMarket, Map::Oman];
        ALL.iter()
    }
}

impl Display for Map {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.pretty())
    }
}

impl RconEncoding for Map {
    fn rcon_encode(&self) -> AsciiString {
        match self {
            // not very pretty but it works?
            Map::Other(str) => str.clone(),
            Map::Metro => AsciiString::from_str("XP0_Metro").unwrap(),
            Map::Locker => AsciiString::from_str("MP_Prison").unwrap(),
            Map::PearlMarket => AsciiString::from_str("XP3_MarketPl").unwrap(),
            Map::Oman => AsciiString::from_str("XP0_Oman").unwrap(),
        }
    }

    fn rcon_decode(ascii: &AsciiStr) -> RconResult<Self> {
        Ok(match ascii.as_str() {
            "XP0_Metro" => Map::Metro,
            "MP_Prison" => Map::Locker,
            "XP3_MarketPl" => Map::PearlMarket,
            "XP0_Oman" => Map::Oman,
            _ => Map::Other(ascii.to_owned()),
        })
    }
}
