use std::fmt::{Display, Formatter};

use ascii::AsciiString;

use super::{
    ea_guid::Eaid,
    visibility::{Team, Visibility},
};

/// Maybe some flyweight or proxy, to enable `.kill()`, getting EA GUID, etc?
#[derive(Debug, Clone)]
pub struct Player {
    pub name: AsciiString,
    pub eaid: Eaid,
}

impl Display for Player {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name.as_str())
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug)]
pub enum Map {
    Other(AsciiString),
}

#[derive(Debug)]
pub enum GameMode {
    Rush,
    Other(AsciiString),
}

#[derive(Debug, Clone)]
pub enum Event {
    Chat {
        vis: Visibility,
        player: Player,
        msg: AsciiString,
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
    PunkBusterMessage(String),
}
