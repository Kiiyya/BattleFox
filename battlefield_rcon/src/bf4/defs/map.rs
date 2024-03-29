use std::{cmp::min, collections::HashMap, str::FromStr};

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

    ZavodNight,
    Outbreak,
    DragonValley,
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
            Map::LancangDam => "Lancang Dam",
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
            Map::Propaganda => "Propaganda",
            Map::Lumphini => "Lumphini",
            Map::SunkenDragon => "Sunken Dragon",
            Map::Whiteout => "Whiteout",
            Map::Hammerhead => "Hammerhead",
            Map::Hangar21 => "Hangar 21",
            Map::Karelia => "Karelia",
            Map::ZavodNight => "Zavod Night",
            Map::Outbreak => "Outbreak",
            Map::DragonValley => "Dragon Valley",
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
        static ALL: [Map; 33] = [
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
            Map::DragonValley,
            Map::ZavodNight,
            Map::Outbreak,
        ];
        ALL.iter()
    }

    /// Gets a short string which will be rendered in BF4 as constant length.
    /// Useful for e.g. indentations, or tables, etc.
    /// The maximum ASCII character count is 11.
    pub fn map_constlen_tabbed(&self) -> &'static str {
        match self {
            Map::Zavod => "Zavod\t",
            Map::Locker => "Locker\t",
            Map::Metro => "Metro\t",
            Map::Propaganda => "Propa\t",
            Map::PearlMarket => "Pearl\t\t",
            Map::LancangDam => "Lancang\t",
            Map::FloodZone => "Flood\t",
            Map::GolmudRailway => "Golmud\t",
            Map::ParacelStorm => "Parcel\t",
            Map::HainanResort => "Hainan\t",
            Map::Shanghai => "Shanghai\t",
            Map::RogueTransmission => "Rogue\t",
            Map::Dawnbreaker => "Dawn\t",
            Map::SilkRoad => "Silk Road\t",
            Map::Altai => "Altai\t\t",
            Map::GuilinPeaks => "Guilin\t",
            Map::DragonPass => "Dr. Pass\t",
            Map::Caspian => "Caspian\t",
            Map::Firestorm => "Firestorm\t",
            Map::Oman => "Oman\t",
            Map::LostIslands => "Lost Isl.\t",
            Map::NanshaStrike => "Nansha\t",
            Map::WaveBreaker => "Wavebr.\t",
            Map::OpMortar => "Mortar\t",
            Map::Lumphini => "Lumphi\t",
            Map::SunkenDragon => "Sunken\t",
            Map::Whiteout => "Whiteout\t",
            Map::Hammerhead => "Hammer\t",
            Map::Hangar21 => "Hangar\t",
            Map::Karelia => "Karelia\t",
            Map::ZavodNight => "Z. Night\t",
            Map::Outbreak => "Outbreak\t",
            Map::DragonValley => "Dr. Valley\t",
        }
    }

    // /// Given a prefix length, returns `map.short()` but with the first `prefixlen` letters
    // /// capitalized, and tabs appended so that the visual length is always 3 tabs for all
    // /// combinations of `map` and `prefixlen`.
    // /// Or, well, I tested it ingame with prefixlengths 1, 2, 3. Other than that I give no
    // /// guarantee.
    // ///
    // /// For example,
    // /// `Map::Pearl.tabconstlen_prefixlen(3) = "PEArl\t\t"`.
    // pub fn tab3_prefixlen(&self, prefixlen: usize) -> String {
    //     let ntabs = match (self, prefixlen) {
    //         (Map::Zavod, _) => 1,
    //         (Map::LancangDam, _) => 1,
    //         (Map::FloodZone, 0) => 2,
    //         (Map::FloodZone, _) => 1,
    //         (Map::GolmudRailway, _) => 1,
    //         (Map::ParacelStorm, _) => 1,
    //         (Map::Locker, _) => 1,
    //         (Map::HainanResort, _) => 1,
    //         (Map::Shanghai, _) => 1,
    //         (Map::RogueTransmission, _) => 1,
    //         (Map::Dawnbreaker, _) => 1,
    //         (Map::SilkRoad, _) => 2,
    //         (Map::Altai, n) if n >= 3 => 1,
    //         (Map::Altai, _) => 2,
    //         (Map::GuilinPeaks, _) => 1,
    //         (Map::DragonPass, _) => 1,
    //         (Map::Caspian, _) => 1,
    //         (Map::Firestorm, _) => 1,
    //         (Map::Metro, _) => 1,
    //         (Map::Oman, _) => 1,
    //         (Map::LostIslands, _) => 1,
    //         (Map::NanshaStrike, _) => 1,
    //         (Map::WaveBreaker, _) => 1,
    //         (Map::OpMortar, _) => 1,
    //         (Map::PearlMarket, n) if n >= 3 => 1,
    //         (Map::PearlMarket, _) => 2,
    //         (Map::Propaganda, _) => 1,
    //         (Map::Lumphini, _) => 1,
    //         (Map::SunkenDragon, _) => 1,
    //         (Map::Whiteout, _) => 1,
    //         (Map::Hammerhead, _) => 1,
    //         (Map::Hangar21, _) => 1,
    //         (Map::Karelia, _) => 1,
    //         (Map::ZavodNight, _) => 1,
    //         (Map::Outbreak, _) => 1,
    //         (Map::DragonValley, _) => 1,
    //     };

    //     let mut upper = self.short()[..prefixlen].to_ascii_uppercase();
    //     let lower = self.short()[prefixlen..].to_string();
    //     upper += &lower;
    //     upper += &"\t".repeat(ntabs);
    //     upper
    // }

    /// Given a prefix length, returns `map.short()` but with the first `prefixlen` letters
    /// capitalized, and tabs appended so that the visual length is 4 tabs for all
    /// combinations of `map` and `prefixlen`.
    /// Or, well, I tested it ingame with prefixlengths 0 to 6. Other than that I give no
    /// guarantee.
    ///
    /// For example,
    /// `Map::Pearl.tabconstlen_prefixlen(3) = "PEArl\t\t"`.
    pub fn tab4_prefixlen(&self, prefixlen: usize) -> String {
        let ntabs = match (self, prefixlen) {
            (Map::Zavod, _) => 2,
            (Map::LancangDam, _) => 2,
            (Map::FloodZone, 0) => 3,
            (Map::FloodZone, _) => 2,
            (Map::GolmudRailway, _) => 2,
            (Map::ParacelStorm, _) => 2,
            (Map::Locker, _) => 2,
            (Map::HainanResort, _) => 2,
            (Map::Shanghai, _) => 2,
            (Map::RogueTransmission, _) => 2,
            (Map::Dawnbreaker, _) => 2,
            (Map::SilkRoad, _) => 3,
            (Map::Altai, n) if n >= 3 => 2,
            (Map::Altai, _) => 3,
            (Map::GuilinPeaks, _) => 2,
            (Map::DragonPass, _) => 1,
            (Map::Caspian, _) => 2,
            (Map::Firestorm, n) if n >= 5 => 1,
            (Map::Firestorm, _) => 2,
            (Map::Metro, _) => 2,
            (Map::Oman, _) => 2,
            (Map::LostIslands, _) => 1,
            (Map::NanshaStrike, _) => 2,
            (Map::WaveBreaker, _) => 1,
            (Map::OpMortar, _) => 2,
            (Map::PearlMarket, n) if n >= 3 => 2,
            (Map::PearlMarket, _) => 3,
            (Map::Propaganda, _) => 2,
            (Map::Lumphini, _) => 2,
            (Map::SunkenDragon, _) => 2,
            (Map::Whiteout, _) => 2,
            (Map::Hammerhead, _) => 2,
            (Map::Hangar21, _) => 2,
            (Map::Karelia, _) => 2,
            (Map::ZavodNight, 0) => 3,
            (Map::ZavodNight, _) => 2,
            (Map::Outbreak, n) if n >= 5 => 1,
            (Map::Outbreak, _) => 2,
            (Map::DragonValley, _) => 2,
        };

        let split = min(prefixlen, self.short().len()); // to prevent index out of bounds.
        let mut upper = self.short()[..split].to_ascii_uppercase();
        let lower = self.short()[split..].to_string();
        upper += &lower;
        upper += &"\t".repeat(ntabs);
        upper
    }

    /// Given a prefix length, returns `map.short()` but with the first `prefixlen` letters
    /// capitalized, and tabs appended so that the visual length is 4 tabs for all
    /// combinations of `map` and `prefixlen`.
    /// Or, well, I tested it ingame with prefixlengths 0 to 6. Other than that I give no
    /// guarantee.
    ///
    /// For example,
    /// `Map::Pearl.tabconstlen_prefixlen(3) = "PEArl\t\t"`.
    pub fn tab4_prefixlen_wvehicles(&self, prefixlen: usize, vehicles: bool) -> String {
        let ntabs = match (self, prefixlen, vehicles) {
            (Map::Zavod, _, true) => 2,
            (Map::Zavod, _, _) => 1,
            (Map::LancangDam, _, true) => 2,
            (Map::LancangDam, _, _) => 1,
            (Map::FloodZone, 0, true) => 3,
            (Map::FloodZone, 0, false) => 2,
            (Map::FloodZone, _, true) => 2,
            (Map::FloodZone, n, false) if n >= 2 => 1,
            (Map::FloodZone, _, false) => 2,
            (Map::GolmudRailway, _, true) => 2,
            (Map::GolmudRailway, _, _) => 1,
            (Map::ParacelStorm, _, true) => 2,
            (Map::ParacelStorm, _, _) => 1,
            (Map::Locker, _, true) => 2,
            (Map::Locker, _, _) => 1,
            (Map::HainanResort, _, true) => 2,
            (Map::HainanResort, _, _) => 1,
            (Map::Shanghai, _, true) => 2,
            (Map::Shanghai, _, _) => 1,
            (Map::RogueTransmission, _, true) => 2,
            (Map::RogueTransmission, 0, false) => 2,
            (Map::RogueTransmission, _, _) => 1,
            (Map::Dawnbreaker, n, false) if n >= 2 => 1,
            (Map::Dawnbreaker, _, _) => 2,
            (Map::SilkRoad, _, true) => 3,
            (Map::SilkRoad, _, _) => 2,
            (Map::Altai, n, _) if n >= 3 => 2,
            (Map::Altai, _, true) => 3,
            (Map::Altai, _, _) => 2,
            (Map::GuilinPeaks, n, false) if n >= 4 => 1,
            (Map::GuilinPeaks, _, _) => 2,
            (Map::DragonPass, _, true) => 2,
            (Map::DragonPass, _, _) => 1,
            (Map::Caspian, _, true) => 2,
            (Map::Caspian, _, _) => 1,
            (Map::Firestorm, n, _) if n >= 5 => 1,
            (Map::Firestorm, _, true) => 2,
            (Map::Firestorm, _, _) => 1,
            (Map::Metro, n, false) if n >= 3 => 1,
            (Map::Metro, _, _) => 2,
            (Map::Oman, n, false) if n >= 4 => 1,
            (Map::Oman, _, _) => 2,
            (Map::LostIslands, _, true) => 2,
            (Map::LostIslands, _, _) => 1,
            (Map::NanshaStrike, _, true) => 2,
            (Map::NanshaStrike, _, _) => 1,
            (Map::WaveBreaker, _, true) => 2,
            (Map::WaveBreaker, _, _) => 1,
            (Map::OpMortar, _, true) => 2,
            (Map::OpMortar, _, _) => 1,
            (Map::PearlMarket, n, true) if n >= 3 => 2,
            (Map::PearlMarket, _, true) => 3,
            (Map::PearlMarket, n, false) if n >= 4 => 1,
            (Map::PearlMarket, _, _) => 2,
            (Map::Propaganda, n, false) if n >= 2 => 1,
            (Map::Propaganda, _, _) => 2,
            (Map::Lumphini, _, true) => 2,
            (Map::Lumphini, _, _) => 1,
            (Map::SunkenDragon, _, true) => 2,
            (Map::SunkenDragon, _, _) => 1,
            (Map::Whiteout, _, true) => 2,
            (Map::Whiteout, _, _) => 1,
            (Map::Hammerhead, _, true) => 2,
            (Map::Hammerhead, _, _) => 1,
            (Map::Hangar21, _, true) => 2,
            (Map::Hangar21, _, _) => 1,
            (Map::Karelia, _, true) => 2,
            (Map::Karelia, _, _) => 1,
            (Map::ZavodNight, 0, true) => 3,
            (Map::ZavodNight, _, _) => 2,
            (Map::Outbreak, n, _) if n >= 5 => 1,
            (Map::Outbreak, _, true) => 2,
            (Map::Outbreak, _, _) => 1,
            (Map::DragonValley, n, false) if n >= 1 => 1,
            (Map::DragonValley, _, _) => 2,
        };

        let split = min(prefixlen, self.short().len()); // to prevent index out of bounds.
        let mut upper = self.short()[..split].to_ascii_uppercase();
        let lower = self.short()[split..].to_string();
        upper += &lower;
        if !vehicles {
            upper += "[INF]";
        }
        upper += &"\t".repeat(ntabs);
        upper
    }

    pub fn short_names(&self) -> std::slice::Iter<'static, &'static str> {
        match self {
            Map::Metro => {
                static ALL: [&str; 6] = [
                    "metro",
                    "operationmetro",
                    "operation_metro",
                    "operation-metro",
                    "metr",
                    "met"
                ];
                ALL.iter()
            }
            Map::Locker => {
                static ALL: [&str; 7] = [
                    "locker",
                    "lock",
                    "operationlocker",
                    "operation_locker",
                    "operation-locker",
                    "loc",
                    "lok"
                ];
                ALL.iter()
            }
            Map::PearlMarket => {
                static ALL: [&str; 8] = [
                    "pearl",
                    "pear",
                    "paerl",
                    "market",
                    "pearlmarket",
                    "pearl_market",
                    "pearl-market",
                    "pea"
                ];
                ALL.iter()
            }
            Map::Oman => {
                static ALL: [&str; 13] = [
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
                    "omn",
                    "oma"
                ];
                ALL.iter()
            }
            Map::Zavod => {
                static ALL: [&str; 3] = ["zavod", "zav", "zavo"];
                ALL.iter()
            }
            Map::LancangDam => {
                static ALL: [&str; 7] =
                    ["lancang", "lanc", "langanc", "lancng", "lancangdam", "dam", "lan"];
                ALL.iter()
            }
            Map::FloodZone => {
                static ALL: [&str; 5] = ["flood", "floodzone", "zone", "flo", "zon"];
                ALL.iter()
            }
            Map::GolmudRailway => {
                static ALL: [&str; 11] = [
                    "golmud",
                    "golmod",
                    "golmund",
                    "rail",
                    "railway",
                    "golmud_railway",
                    "golmudrailway",
                    "golmud-railway",
                    "golm",
                    "gol",
                    "rai"
                ];
                ALL.iter()
            }
            Map::ParacelStorm => {
                static ALL: [&str; 12] = [
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
                    "par",
                ];
                ALL.iter()
            }
            Map::HainanResort => {
                static ALL: [&str; 10] = [
                    "hainan",
                    "hainen",
                    "heinen",
                    "heinan",
                    "resort",
                    "hainanresort",
                    "hainan-resort",
                    "hainan_resort",
                    "hai",
                    "res",
                ];
                ALL.iter()
            }
            Map::Shanghai => {
                static ALL: [&str; 9] = [
                    "shang",
                    "shanghai",
                    "shangai",
                    "shangay",
                    "siege",
                    "siegeshanghai",
                    "siegeofshanghai",
                    "sha",
                    "sie,"
                ];
                ALL.iter()
            }
            Map::RogueTransmission => {
                static ALL: [&str; 14] = [
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
                    "rog",
                    "rou",
                    "tra",
                ];
                ALL.iter()
            }
            Map::Dawnbreaker => {
                static ALL: [&str; 5] = ["dawn", "dawnbreaker", "breaker", "dawnbreak", "daw"];
                ALL.iter()
            }
            Map::SilkRoad => {
                static ALL: [&str; 5] = ["silk", "silkroad", "silk-road", "silk_road", "sil"];
                ALL.iter()
            }
            Map::Altai => {
                static ALL: [&str; 6] =
                    ["altai", "altei", "altairange", "altai-range", "altai_range", "alt"];
                ALL.iter()
            }
            Map::GuilinPeaks => {
                static ALL: [&str; 12] = [
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
                    "gui",
                ];
                ALL.iter()
            }
            Map::DragonPass => {
                static ALL: [&str; 6] = [
                    "drgnpass",
                    "drpass",
                    "dragon-pass",
                    "dragon_pass",
                    "dpass",
                    "drp",
                ];
                ALL.iter()
            }
            Map::Caspian => {
                static ALL: [&str; 8] = [
                    "caspian",
                    "caspianborder",
                    "caspian-border",
                    "caspian_border",
                    "caspien",
                    "caspain",
                    "casp",
                    "cas",
                ];
                ALL.iter()
            }
            Map::Firestorm => {
                static ALL: [&str; 4] = ["firestorm", "fireform", "fire", "fir"];
                ALL.iter()
            }
            Map::LostIslands => {
                static ALL: [&str; 11] = [
                    "lostislnd",
                    "lost",
                    "losti",
                    "lost-islands",
                    "lost_islands",
                    "islands",
                    "lostisland",
                    "lost-island",
                    "lost_island",
                    "los",
                    "isl",
                ];
                ALL.iter()
            }
            Map::NanshaStrike => {
                static ALL: [&str; 5] =
                    ["nansha", "nanshastrike", "nansha-strike", "nansha_strike", "nan"];
                ALL.iter()
            }
            Map::WaveBreaker => {
                static ALL: [&str; 5] = ["wavebrkr", "wave", "wave_breaker", "wave-breaker", "wav"];
                ALL.iter()
            }
            Map::OpMortar => {
                static ALL: [&str; 9] = [
                    "mortar",
                    "operationmortar",
                    "operation_mortar",
                    "operation-mortar",
                    "morter",
                    "opmortar",
                    "op-mortar",
                    "op_mortar",
                    "mor",
                ];
                ALL.iter()
            }
            Map::Propaganda => {
                static ALL: [&str; 7] =
                    ["propa", "prop", "propaganda", "propapanda", "propagaynda", "prp", "pro"];
                ALL.iter()
            }
            Map::Lumphini => {
                static ALL: [&str; 10] = [
                    "lumphini",
                    "lump",
                    "garden",
                    "lumphinigarden",
                    "lumphini_garden",
                    "lumphini-garden",
                    "lumpini",
                    "lumfini",
                    "lumpi",
                    "lum",
                ];
                ALL.iter()
            }
            Map::SunkenDragon => {
                static ALL: [&str; 8] = [
                    "sunken",
                    "sunk",
                    "sunkendragon",
                    "sunkendragen",
                    "sunken_dragon",
                    "sunken-dragon",
                    "sdragon",
                    "sun",
                ];
                ALL.iter()
            }
            Map::Whiteout => {
                static ALL: [&str; 6] = [
                    "whiteout",
                    "white",
                    "operationwhiteout",
                    "operation-whiteout",
                    "operation_whiteout",
                    "whi",
                ];
                ALL.iter()
            }
            Map::Hammerhead => {
                static ALL: [&str; 5] = ["hammer", "hammerhead", "hammer_head", "hammer-head", "ham"];
                ALL.iter()
            }
            Map::Hangar21 => {
                static ALL: [&str; 6] = ["hangar", "hanger", "hangar21", "hangar-21", "hangar_21", "han"];
                ALL.iter()
            }
            Map::Karelia => {
                static ALL: [&str; 8] = [
                    "karelia",
                    "giants",
                    "kare",
                    "karelie",
                    "giantsof",
                    "giantsofkarelia",
                    "gia",
                    "kar",
                ];
                ALL.iter()
            }
            Map::ZavodNight => {
                static ALL: [&str; 11] = [
                    "night",
                    "grave",
                    "graveyard",
                    "gravey",
                    "yard",
                    "gyard",
                    "zavodgraveyard",
                    "zavodgrave",
                    "nightshift",
                    "nightzavod",
                    "zni",
                ];
                ALL.iter()
            }
            Map::Outbreak => {
                static ALL: [&str; 3] = [
                    "outbreak",
                    "cmp",
                    "out",
                ];
                ALL.iter()
            }
            Map::DragonValley => {
                static ALL: [&str; 6] = [
                    "valley",
                    "dragonvalley",
                    "dragon_valley",
                    "dragon-valley",
                    "val",
                    "vall",
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

            "XP7_Valley" => Map::DragonValley,
            "XP6_CMP" => Map::Outbreak,
            "XP5_Night_01" => Map::ZavodNight,
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

            Map::ZavodNight => "XP5_Night_01",
            Map::Outbreak => "XP6_CMP",
            Map::DragonValley => "XP7_Valley",
        };

        AsciiString::from_str(str).unwrap()
    }
}
