use std::{collections::HashMap, fmt::{Display, Formatter}, str::FromStr};

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
            Map::Other(_) => "(Some unknown map)", // FIXME some day we won't have "Other".
        }
    }

    /// Gets an iterator over all currently known maps. (Excluding `Other(...)`).
    pub fn all() -> std::slice::Iter<'static, Map> {
        static ALL: [Map; 4] = [Map::Metro, Map::Locker, Map::PearlMarket, Map::Oman];
        ALL.iter()
    }

    pub fn short_names(&self) -> std::slice::Iter<'static, &str> {
        match self {
            Map::Metro => {
                static ALL: [&str; 4] = ["metro", "operationmetro", "operation_metro", "operation-metro"];
                ALL.iter()
            }
            Map::Locker => {
                static ALL: [&str; 4] = ["locker", "operationlocker", "operation_locker", "operation-locker"];
                ALL.iter()
            }
            Map::PearlMarket => {
                static ALL: [&str; 5] = ["pearl", "market", "pearlmarket", "pearl_market", "pearl-market"];
                ALL.iter()
            }
            Map::Oman => {
                static ALL: [&str; 11] = ["oman", "gulfofoman", "gulf", "gulfoman", "omangulf", "gulf_oman", "gulf-oman", "oman_gulf", "oman-gulf", "gulf_of_oman", "gulf-of-oman"];
                ALL.iter()
            }
            Map::Other(_) => {
                static ALL: [&str; 0] = [];
                ALL.iter()
            }
        }
    }

    /// - "pearl" -> `Map::PearlMarket`,
    /// - "market" -> `Map::PearlMarket`,
    /// - "locker" -> `Map::Locker`,
    /// - etc..
    pub fn try_from_short<'a>(str: impl Into<&'a str>) -> Option<Map> {
        // TODO: use strsim.
        MAP_SHORTNAMES.get(str.into()).cloned()
    }

    // pub fn all_short_names() -> &'static HashMap<&'static str, Map> {
    //     static shortnames : HashMap<&'static str, Map> = HashMap::new();
    //     for map in Map::all() {
    //         for shortname in map.short_names() {
    //             shortnames.insert(shortname, map.clone());
    //         }
    //     }

    //     &shortnames
    // }
}

lazy_static! {
    /// - "pearl" -> `Map::PearlMarket`,
    /// - "market" -> `Map::PearlMarket`,
    /// - "locker" -> `Map::Locker`,
    /// - etc..
    pub static ref MAP_SHORTNAMES: HashMap<&'static str, Map> = {
        let mut shortnames : HashMap<&'static str, Map> = HashMap::new();
        for map in Map::all() {
            for shortname in map.short_names() {
                shortnames.insert(shortname, map.clone());
            }
        }
        shortnames
    };
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
