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
    Other(AsciiString),
}

impl Display for Weapon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Weapon::Other(ascii) => f.write_str(ascii.as_str()),
            Weapon::Mortar => f.write_str("Mortar"),
            Weapon::Ucav => f.write_str("UCAV"),
        }
    }
}

impl RconDecoding for Weapon {
    fn rcon_decode(ascii: &AsciiStr) -> RconResult<Self> {
        Ok(match ascii.as_str() {
            "M224" | "U_M224" => Weapon::Mortar,
            "XP1/Gameplay/Gadgets/UCAV/UCAV_Launcher" | "UCAV" => Weapon::Ucav,
            _ => Weapon::Other(ascii.to_owned()),
        })
    }
}

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
