//! Various definitions partaining to Battlefield 4, such as
//! - Maps
//! - Squad, Team, Visibility, etc.
//! - Events for Bf4 (such as Kill, Chat, etc).

use super::{ea_guid::Eaid, player_info_block::PlayerInfo, RconDecoding, RconEncoding};
use crate::rcon::{RconError, RconResult};
use ascii::{AsciiStr, AsciiString};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

pub mod map;
pub mod commorose;
pub use commorose::{CommmoRose};
pub use map::Map;
pub mod vis;
pub use vis::{Squad, Team, Visibility};

/////////////////////////////////////////////////////////////////////
/////////////////////// Player //////////////////////////////////////
/////////////////////////////////////////////////////////////////////

// Maybe make it some flyweight or proxy, to enable `.kill()`, getting EA GUID, etc?
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Player {
    pub name: AsciiString,
    pub eaid: Eaid,
}

// pub const PLAYER_SERVER : Player = Player {
//     name: AsciiString::from_str("Server").unwrap(),
//     eaid: EAID_SERVER,
// };

impl Display for Player {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name.as_str())
    }
}

impl From<&Player> for AsciiString {
    fn from(pl: &Player) -> Self {
        pl.name.clone()
    }
}

// impl Into<AsciiString> for Player {
//     fn into(self) -> AsciiString {
//         self.name
//     }
// }

/////////////////////////////////////////////////////////////////////
/////////////////////// Weapon //////////////////////////////////////
/////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Weapon {
    Mortar,
    Ucav,

    Lav,
    Tank,

    /// Rhib, .50cal buggy, MRAP, etc.
    ArmedTransport,
    /// Jetski, Hovercraft,  Bike, Quadbike, Snowmobile, Skidloader
    UnarmoredTransport,
    Amtrac,
    TransportChopper,

    M67,
    Incendiary,
    RGO,
    V40,
    SlamMine,
    C4,
    Claymore,

    M320,
    M320LVG,

    Roadkill,

    Other(AsciiString),
}

impl Display for Weapon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Weapon::*;
        match self {
            Weapon::Other(ascii) => f.write_str(ascii.as_str()),
            Weapon::Mortar => f.write_str("Mortar"),
            Weapon::Ucav => f.write_str("UCAV"),

            Weapon::Lav => f.write_str("Lav"),
            Weapon::Tank => f.write_str("Tank"),

            Weapon::ArmedTransport => f.write_str("ArmedTransport"),
            Weapon::UnarmoredTransport => f.write_str("UnarmoredTransport"),
            Amtrac => f.write_str("Amtrac"),
            TransportChopper => f.write_str("TransportChopper"),

            Weapon::M67 => f.write_str("M67"),
            Weapon::Incendiary => f.write_str("Incendiary"),
            Weapon::M320 => f.write_str("M320"),
            Weapon::M320LVG => f.write_str("M320LVG"),

            RGO => f.write_str("RGO"),
            V40 => f.write_str("V40"),
            SlamMine => f.write_str("SlamMine"),
            C4 => f.write_str("C4"),
            Claymore => f.write_str("Claymore"),
            Roadkill => f.write_str("Roadkill"),
        }
    }
}

impl RconDecoding for Weapon {
    fn rcon_decode(ascii: &AsciiStr) -> RconResult<Self> {
        use Weapon::*;
        Ok(match ascii.as_str() {
            "M224" | "U_M224" => Mortar,
            "XP1/Gameplay/Gadgets/UCAV/UCAV_Launcher" | "UCAV" => Ucav,

            "XP2/Gameplay/Vehicles/PatrolHovercraft/PatrolHovercraft" | "XP1/Gameplay/Vehicles/KLR650/KLR650" | "Gameplay/Vehicles/PWC_JetSki/PWC_JetSki"
            | "Gameplay/Vehicles/QuadBike/QUADBIKE" | "Gameplay/Vehicles/QuadBike/spec/QUADBIKE_Night" | "XP0/Gameplay/Vehicles/SkidLoader/SkidLoader"
            | "XP4/Gameplay/Vehicles/Snowmobile/Snowmobile" => UnarmoredTransport,


            "DPV" | "VDV Buggy" | "Gameplay/Vehicles/US_MRAP-COUGAR/spec/US_MRAP-COUGAR_Night" | "Gameplay/Vehicles/US_MRAP-COUGAR/US_MRAP-COUGAR" | "RHIB"
            | "Gameplay/Vehicles/RU_MRAP_SPM3/spec/RU_MRAP_SPM3_Night" | "Gameplay/Vehicles/RU_MRAP_SPM3/RU_MRAP_SPM3" | "Gameplay/Vehicles/CH_MRAP-ZFB-05/CH_MRAP-ZFB-05" => ArmedTransport,

            "Gameplay/Vehicles/AAV-7A1/AAV-7A1" => Amtrac,

            "Ka-60" | "Gameplay/Vehicles/CH_LTHE_Z-9/CH_LTHE_Z-9" => TransportChopper,

            "Gameplay/Vehicles/M1A2/M1Abrams" | "Gameplay/Vehicles/M1A2/spec/M1Abrams_Night" | "T90" | "Gameplay/Vehicles/CH_MBT_Type99/CH_MBT_Type99" => Tank,

            "Gameplay/Vehicles/CH_AA_PGZ-95/CH_AA_PGZ-95" | "Gameplay/Vehicles/BTR-90/BTR90" | "Gameplay/Vehicles/BTR-90/spec/BTR90_Night" | "Gameplay/Vehicles/CH_IFV_ZBD09/CH_IFV_ZBD09"
            | "Gameplay/Vehicles/HIMARS/HIMARS" | "Gameplay/Vehicles/HIMARS/spec/HIMARS_Night" | "Gameplay/Vehicles/LAV25/LAV_AD" | "Gameplay/Vehicles/LAV25/LAV25"
            | "Gameplay/Vehicles/LAV25/spec/LAV25_Night" | "Gameplay/Vehicles/9K22_Tunguska_M/9K22_Tunguska_M" => Lav,

            "U_M67" => M67,
            "U_Grenade_RGO" => RGO,
            "U_M34" => Incendiary,
            "U_V40" => V40,
            "U_SP_Claymore" | "U_Claymore_Recon" | "U_Claymore" => Claymore,
            "U_C4" | "U_C4_Support" => C4,
            "U_SLAM" => SlamMine,

            "RoadKill" => Roadkill,

            _ => Other(ascii.to_owned()),
        })
    }
}

// impl Weapon {
//     pub fn short_name(&self) -> &str {
//         match self {
//             Weapon::Mortar => "Mortar",
//             Weapon::Ucav => "UCAV",
//             Weapon::Other(s) => s.as_str(),
//             _ =>
//         }
//     }
// }

/////////////////////////////////////////////////////////////////////
/////////////////////// GameMode ////////////////////////////////////
/////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameMode {
    Rush,
    Other(AsciiString),
}

impl Display for GameMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GameMode::Rush => write!(f, "Rush"),
            GameMode::Other(s) => write!(f, "{}", s),
        }
    }
}

impl RconDecoding for GameMode {
    fn rcon_decode(ascii: &AsciiStr) -> RconResult<Self> {
        Ok(match ascii.as_str() {
            "RushLarge0" => GameMode::Rush,
            _ => GameMode::Other(ascii.to_owned()),
        })
    }
}

impl RconEncoding for GameMode {
    fn rcon_encode(&self) -> AsciiString {
        match self {
            GameMode::Other(str) => str.clone(),
            GameMode::Rush => AsciiString::from_str("RushLarge0").unwrap(),
        }
    }
}

/////////////////////////////////////////////////////////////////////
/////////////////////// Event ///////////////////////////////////////
/////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Event {
    Chat {
        vis: Visibility,
        player: Player,
        msg: AsciiString,
    },
    ServerChat {
        msg: AsciiString,
        vis: Visibility,
    },
    Kill {
        killer: Option<Player>,
        weapon: Weapon,
        victim: Player,
        headshot: bool,
    },
    Spawn {
        player: Player,
        team: Team,
    },
    RoundOver {
        winning_team: Team,
    },
    Join {
        player: Player,
    },
    Authenticated {
        player: Player,
    },
    Leave {
        player: Player,
        final_scores: PlayerInfo,
    },
    Disconnect {
        player: AsciiString,
        reason: String,
    },
    TeamChange {
        player: Player,
        team: Team,
        squad: Squad,
    },
    SquadChange {
        player: Player,
        team: Team,
        squad: Squad,
    },
    PunkBusterMessage(String),
    LevelLoaded {
        level_name: Map,
        game_mode: GameMode,
        rounds_played: i32,
        rounds_total: i32
    },
}

#[derive(Debug)]
pub enum Preset {
    Custom,
    Hardcore,
    Normal,
}

impl RconDecoding for Preset {
    fn rcon_decode(ascii: &AsciiStr) -> RconResult<Self> {
        match ascii.as_str().to_lowercase().as_str() {
            "hardcore" => Ok(Self::Hardcore),
            "custom" => Ok(Self::Custom),
            "normal" => Ok(Self::Normal),
            _ => Err(RconError::other("Unknown preset type")),
        }
    }
}

impl RconEncoding for Preset {
    fn rcon_encode(&self) -> AsciiString {
        match self {
            Preset::Custom => AsciiString::from_str("CUSTOM").unwrap(),
            Preset::Hardcore => AsciiString::from_str("HARDCORE").unwrap(),
            Preset::Normal => AsciiString::from_str("NORMAL").unwrap(),
        }
    }
}
