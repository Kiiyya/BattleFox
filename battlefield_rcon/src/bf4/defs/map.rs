use std::{collections::HashMap, str::FromStr};

use ascii::{AsciiStr, AsciiString};
use serde::{Deserialize, Serialize};

use crate::{
    bf4::{RconDecoding, RconEncoding},
    rcon::{RconError, RconResult},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Map {
    Zavod,
    LancangDam,
    FloodZone,
    GolmudRailway,
    ParacelStorm,
    Locker,
    HainanResort,
    Shanghai,
    RogueTransmission,
    Dawnbreaker,

    SilkRoad,
    Altai,
    GuilinPeaks,
    DragonPass,

    Caspian,
    Firestorm,
    Metro,
    Oman,

    LostIslands,
    NanshaStrike,
    WaveBreaker,
    OpMortar,

    PearlMarket,
    Propaganda,
    Lumphini,
    SunkenDragon,

    Whiteout,
    Hammerhead,
    Hangar21,
    Karelia,
    // Other(AsciiString),
}

impl Map {
    /// Long, uppercase, pretty names.
    /// "Pearl Market", "Zavod 331", etc...
    #[allow(non_snake_case)]
    pub fn Pretty(&self) -> &'static str {
        match self {
            Map::Metro => "Metro",
            Map::Locker => "Locker",
            Map::PearlMarket => "Pearl Market",
            Map::Oman => "Gulf of Oman",
            Map::Zavod => "Zavod 311",
            Map::LancangDam => "Lancang Damn",
            Map::FloodZone => "Flood Zone",
            Map::GolmudRailway => "Golmud",
            Map::ParacelStorm => "Paracel",
            Map::HainanResort => "Hainan",
            Map::Shanghai => "Shanghai",
            Map::RogueTransmission => "Rogue",
            Map::Dawnbreaker => "Dawnbreaker",
            Map::SilkRoad => "Silk Road",
            Map::Altai => "Altai",
            Map::GuilinPeaks => "Guilin Peaks",
            Map::DragonPass => "Dragon Pass",
            Map::Caspian => "Caspian",
            Map::Firestorm => "Firestorm",
            Map::LostIslands => "Lost Islands",
            Map::NanshaStrike => "Nansha",
            Map::WaveBreaker => "Wave Breaker",
            Map::OpMortar => "Op. Mortar",
            Map::Propaganda => "Propapanda", // don't judge me.
            Map::Lumphini => "Lumphini",
            Map::SunkenDragon => "Sunken Dragon",
            Map::Whiteout => "Whiteout",
            Map::Hammerhead => "Hammerhead",
            Map::Hangar21 => "Hangar 21",
            Map::Karelia => "Karelia",
        }
    }

    /// Any names which are easy to type, single-word, short, etc..
    /// - `pearl`
    /// - `propa`
    /// - `locker`
    pub fn short(&self) -> &'static str {
        // just get the first entry in the `short_names()` list.
        self.short_names().next().unwrap()
    }

    /// Gets an iterator over all currently known maps. (Excluding `Other(...)`).
    pub fn all() -> std::slice::Iter<'static, Map> {
        static ALL: [Map; 30] = [
            Map::Metro,
            Map::Locker,
            Map::PearlMarket,
            Map::Oman,
            Map::Zavod,
            Map::LancangDam,
            Map::FloodZone,
            Map::GolmudRailway,
            Map::ParacelStorm,
            Map::HainanResort,
            Map::Shanghai,
            Map::RogueTransmission,
            Map::Dawnbreaker,
            Map::SilkRoad,
            Map::Altai,
            Map::GuilinPeaks,
            Map::DragonPass,
            Map::Caspian,
            Map::Firestorm,
            Map::LostIslands,
            Map::NanshaStrike,
            Map::WaveBreaker,
            Map::OpMortar,
            Map::Propaganda,
            Map::Lumphini,
            Map::SunkenDragon,
            Map::Whiteout,
            Map::Hammerhead,
            Map::Hangar21,
            Map::Karelia,
        ];
        ALL.iter()
    }

    pub fn short_names(&self) -> std::slice::Iter<'static, &'static str> {
        match self {
            Map::Metro => {
                static ALL: [&str; 4] = [
                    "metro",
                    "operationmetro",
                    "operation_metro",
                    "operation-metro",
                ];
                ALL.iter()
            }
            Map::Locker => {
                static ALL: [&str; 4] = [
                    "locker",
                    "operationlocker",
                    "operation_locker",
                    "operation-locker",
                ];
                ALL.iter()
            }
            Map::PearlMarket => {
                static ALL: [&str; 6] = [
                    "pearl",
                    "paerl",
                    "market",
                    "pearlmarket",
                    "pearl_market",
                    "pearl-market",
                ];
                ALL.iter()
            }
            Map::Oman => {
                static ALL: [&str; 11] = [
                    "oman",
                    "gulfofoman",
                    "gulf",
                    "gulfoman",
                    "omangulf",
                    "gulf_oman",
                    "gulf-oman",
                    "oman_gulf",
                    "oman-gulf",
                    "gulf_of_oman",
                    "gulf-of-oman",
                ];
                ALL.iter()
            }
            Map::Zavod => {
                static ALL: [&str; 2] = ["zavod", "zav"];
                ALL.iter()
            }
            Map::LancangDam => {
                static ALL: [&str; 6] =
                    ["lancang", "lanc", "langanc", "lancng", "lancangdam", "dam"];
                ALL.iter()
            }
            Map::FloodZone => {
                static ALL: [&str; 3] = ["flood", "floodzone", "zone"];
                ALL.iter()
            }
            Map::GolmudRailway => {
                static ALL: [&str; 8] = [
                    "golmud",
                    "golmod",
                    "golmund",
                    "rail",
                    "railway",
                    "golmud_railway",
                    "golmudrailway",
                    "golmud-railway",
                ];
                ALL.iter()
            }
            Map::ParacelStorm => {
                static ALL: [&str; 11] = [
                    "paracel",
                    "parcel",
                    "parc",
                    "para",
                    "paracelstorm",
                    "storm",
                    "parcelstorm",
                    "parcel-storm",
                    "paracel-storm",
                    "paracel_storm",
                    "parcel-storm",
                ];
                ALL.iter()
            }
            Map::HainanResort => {
                static ALL: [&str; 8] = [
                    "hainan",
                    "hainen",
                    "heinen",
                    "heinan",
                    "resort",
                    "hainanresort",
                    "hainan-resort",
                    "hainan_resort",
                ];
                ALL.iter()
            }
            Map::Shanghai => {
                static ALL: [&str; 7] = [
                    "shang",
                    "shanghai",
                    "shangai",
                    "shangay",
                    "siege",
                    "siegeshanghai",
                    "siegeofshanghai",
                ];
                ALL.iter()
            }
            Map::RogueTransmission => {
                static ALL: [&str; 11] = [
                    "rogue",
                    "rouge",
                    "rougue",
                    "trans",
                    "transmission",
                    "roguetransmission",
                    "roguetrans",
                    "rogue_transmission",
                    "rogue-transmission",
                    "rogue_trans",
                    "rogue-trans",
                ];
                ALL.iter()
            }
            Map::Dawnbreaker => {
                static ALL: [&str; 4] = ["dawnbreaker", "dawn", "breaker", "dawnbreak"];
                ALL.iter()
            }
            Map::SilkRoad => {
                static ALL: [&str; 4] = ["silk", "silkroad", "silk-road", "silk_road"];
                ALL.iter()
            }
            Map::Altai => {
                static ALL: [&str; 5] =
                    ["altai", "altei", "altairange", "altai-range", "altai_range"];
                ALL.iter()
            }
            Map::GuilinPeaks => {
                static ALL: [&str; 11] = [
                    "guilin",
                    "guilen",
                    "guilinpeaks",
                    "guilin_peaks",
                    "guilin-peaks",
                    "gpeaks",
                    "geilin",
                    "goilin",
                    "guilean",
                    "guileen",
                    "guipeak",
                ];
                ALL.iter()
            }
            Map::DragonPass => {
                static ALL: [&str; 7] = [
                    "dragonpass",
                    "dragon-pass",
                    "dragon_pass",
                    "dragonass",
                    "dragon_ass", // hehe
                    "dragon-ass",
                    "dpass",
                ];
                ALL.iter()
            }
            Map::Caspian => {
                static ALL: [&str; 7] = [
                    "caspian",
                    "caspianborder",
                    "caspian-border",
                    "caspian_border",
                    "caspien",
                    "caspain",
                    "casp",
                ];
                ALL.iter()
            }
            Map::Firestorm => {
                static ALL: [&str; 3] = ["firestorm", "fireform", "fire"];
                ALL.iter()
            }
            Map::LostIslands => {
                static ALL: [&str; 9] = [
                    "lostislands",
                    "lost",
                    "losti",
                    "lost-islands",
                    "lost_islands",
                    "islands",
                    "lostisland",
                    "lost-island",
                    "lost_island",
                ];
                ALL.iter()
            }
            Map::NanshaStrike => {
                static ALL: [&str; 4] =
                    ["nansha", "nanshastrike", "nansha-strike", "nansha_strike"];
                ALL.iter()
            }
            Map::WaveBreaker => {
                static ALL: [&str; 4] = ["wavebreaker", "wave", "wave_breaker", "wave-breaker"];
                ALL.iter()
            }
            Map::OpMortar => {
                static ALL: [&str; 8] = [
                    "mortar",
                    "operationmortar",
                    "operation_mortar",
                    "operation-mortar",
                    "morter",
                    "opmortar",
                    "op-mortar",
                    "op_mortar",
                ];
                ALL.iter()
            }
            Map::Propaganda => {
                static ALL: [&str; 5] =
                    ["propa", "prop", "propaganda", "propapanda", "propagaynda"];
                ALL.iter()
            }
            Map::Lumphini => {
                static ALL: [&str; 8] = [
                    "lumphini",
                    "lump",
                    "garden",
                    "lumphinigarden",
                    "lumphini_garden",
                    "lumphini-garden",
                    "lumpini",
                    "lumfini",
                ];
                ALL.iter()
            }
            Map::SunkenDragon => {
                static ALL: [&str; 7] = [
                    "sunken",
                    "sunk",
                    "sunkendragon",
                    "sunkendragen",
                    "sunken_dragon",
                    "sunken-dragon",
                    "sdragon",
                ];
                ALL.iter()
            }
            Map::Whiteout => {
                static ALL: [&str; 5] = [
                    "whiteout",
                    "white",
                    "operationwhiteout",
                    "operation-whiteout",
                    "operation_whiteout",
                ];
                ALL.iter()
            }
            Map::Hammerhead => {
                static ALL: [&str; 4] = ["hammer", "hammerhead", "hammer_head", "hammer-head"];
                ALL.iter()
            }
            Map::Hangar21 => {
                static ALL: [&str; 5] = ["hangar", "hanger", "hangar21", "hangar-21", "hangar_21"];
                ALL.iter()
            }
            Map::Karelia => {
                static ALL: [&str; 6] = [
                    "karelia",
                    "giants",
                    "kare",
                    "karelie",
                    "giantsof",
                    "giantsofkarelia",
                ];
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
                shortnames.insert(shortname, *map);
            }
        }
        shortnames
    };
}

// impl Display for Map {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         f.write_str(self.Pretty())
//     }
// }

impl RconDecoding for Map {
    fn rcon_decode(ascii: &AsciiStr) -> RconResult<Self> {
        Ok(match ascii.as_str() {
            "MP_Abandoned" => Map::Zavod,
            "MP_Damage" => Map::LancangDam,
            "MP_Flooded" => Map::FloodZone,
            "MP_Journey" => Map::GolmudRailway,
            "MP_Naval" => Map::ParacelStorm,
            "MP_Prison" => Map::Locker,
            "MP_Resort" => Map::HainanResort,
            "MP_Siege" => Map::Shanghai,
            "MP_TheDish" => Map::RogueTransmission,
            "MP_Tremors" => Map::Dawnbreaker,

            "XP1_001" => Map::SilkRoad,
            "XP1_002" => Map::Altai,
            "XP1_003" => Map::GuilinPeaks,
            "XP1_004" => Map::DragonPass,

            "XP0_Caspian" => Map::Caspian,
            "XP0_Firestorm" => Map::Firestorm,
            "XP0_Metro" => Map::Metro,
            "XP0_Oman" => Map::Oman,

            "XP2_001" => Map::LostIslands,
            "XP2_002" => Map::NanshaStrike,
            "XP2_003" => Map::WaveBreaker,
            "XP2_004" => Map::OpMortar,

            "XP3_MarketPl" => Map::PearlMarket,
            "XP3_Prpganda" => Map::Propaganda,
            "XP3_UrbanGdn" => Map::Lumphini,
            "XP3_WtrFront" => Map::SunkenDragon,

            "XP4_Arctic" => Map::Whiteout,
            "XP4_SubBase" => Map::Hammerhead,
            "XP4_Titan" => Map::Hangar21,
            "XP4_WlkrFtry" => Map::Karelia,
            _ => return Err(RconError::protocol_msg("Unknown map".to_string())),
        })
    }
}

impl RconEncoding for Map {
    fn rcon_encode(&self) -> AsciiString {
        let str = match self {
            Map::Zavod => "MP_Abandoned",
            Map::LancangDam => "MP_Damage",
            Map::FloodZone => "MP_Flooded",
            Map::GolmudRailway => "MP_Journey",
            Map::ParacelStorm => "MP_Naval",
            Map::Locker => "MP_Prison",
            Map::HainanResort => "MP_Resort",
            Map::Shanghai => "MP_Siege",
            Map::RogueTransmission => "MP_TheDish",
            Map::Dawnbreaker => "MP_Tremors",

            Map::SilkRoad => "XP1_001",
            Map::Altai => "XP1_002",
            Map::GuilinPeaks => "XP1_003",
            Map::DragonPass => "XP1_004",

            Map::Caspian => "XP0_Caspian",
            Map::Firestorm => "XP0_Firestorm",
            Map::Metro => "XP0_Metro",
            Map::Oman => "XP0_Oman",

            Map::LostIslands => "XP2_001",
            Map::NanshaStrike => "XP2_002",
            Map::WaveBreaker => "XP2_003",
            Map::OpMortar => "XP2_004",

            Map::PearlMarket => "XP3_MarketPl",
            Map::Propaganda => "XP3_Prpganda",
            Map::Lumphini => "XP3_UrbanGdn",
            Map::SunkenDragon => "XP3_WtrFront",

            Map::Whiteout => "XP4_Arctic",
            Map::Hammerhead => "XP4_SubBase",
            Map::Hangar21 => "XP4_Titan",
            Map::Karelia => "XP4_WlkrFtry",
        };

        AsciiString::from_str(str).unwrap()
    }
}
