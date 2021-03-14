//! Various definitions partaining to Battlefield 4, such as
//! - Maps
//! - Squad, Team, Visibility, etc.
//! - Events for Bf4 (such as Kill, Chat, etc).

use super::{ea_guid::Eaid, RconEncoding};
use crate::rcon::{RconError, RconResult};
use ascii::{AsciiStr, AsciiString};
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

pub mod map;
pub use map::Map;
pub mod vis;
pub use vis::{Squad, Team, Visibility};

/////////////////////////////////////////////////////////////////////
/////////////////////// Player //////////////////////////////////////
/////////////////////////////////////////////////////////////////////

// Maybe make it some flyweight or proxy, to enable `.kill()`, getting EA GUID, etc?
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Weapon {
    Other(AsciiString),
}

impl Display for Weapon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Weapon::Other(ascii) => f.write_str(ascii.as_str()),
        }
    }
}

/////////////////////////////////////////////////////////////////////
/////////////////////// GameMode ////////////////////////////////////
/////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameMode {
    Rush,
    Other(AsciiString),
}

impl RconEncoding for GameMode {
    fn rcon_encode(&self) -> AsciiString {
        match self {
            GameMode::Other(str) => str.clone(),
            GameMode::Rush => AsciiString::from_str("RushLarge0").unwrap(),
        }
    }

    fn rcon_decode(ascii: &AsciiStr) -> RconResult<Self> {
        Ok(match ascii.as_str() {
            "RushLarge0" => GameMode::Rush,
            _ => GameMode::Other(ascii.to_owned()),
        })
    }
}

/////////////////////////////////////////////////////////////////////
/////////////////////// Event ///////////////////////////////////////
/////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    Leave {
        player: AsciiString,
    },
    PunkBusterMessage(String),
}

#[derive(Debug)]
pub enum Preset {
    Custom,
    Hardcore,
    Normal,
}

impl RconEncoding for Preset {
    fn rcon_encode(&self) -> AsciiString {
        match self {
            Preset::Custom => AsciiString::from_str("CUSTOM").unwrap(),
            Preset::Hardcore => AsciiString::from_str("HARDCORE").unwrap(),
            Preset::Normal => AsciiString::from_str("NORMAL").unwrap(),
        }
    }

    fn rcon_decode(ascii: &AsciiStr) -> RconResult<Self> {
        match ascii.as_str().to_lowercase().as_str() {
            "hardcore" => Ok(Self::Hardcore),
            "custom" => Ok(Self::Custom),
            "normal" => Ok(Self::Normal),
            _ => Err(RconError::other("Unknown preset type")),
        }
    }
}
