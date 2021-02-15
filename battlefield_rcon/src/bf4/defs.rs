use super::{ea_guid::Eaid, RconEncoding};
use crate::rcon::{RconError, RconResult};
use ascii::{AsciiStr, AsciiString, IntoAsciiString};
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

// Maybe make it some flyweight or proxy, to enable `.kill()`, getting EA GUID, etc?
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

#[derive(Debug, Clone)]
pub enum Map {
    Other(AsciiString),
}

impl RconEncoding for Map {
    fn rcon_encode(&self) -> AsciiString {
        match self {
            Map::Other(str) => str.clone(),
        }
    }

    fn rcon_decode(ascii: &AsciiStr) -> RconResult<Self> {
        Ok(Self::Other(ascii.to_owned()))
    }
}

#[derive(Debug, Clone)]
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
        Ok(Self::Other(ascii.to_owned()))
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Team {
    Neutral = 0,
    One = 1,
    Two = 2,
}

impl Team {
    pub(crate) fn rcon_encode(self) -> AsciiString {
        (self as usize).to_string().into_ascii_string().unwrap()
    }

    pub(crate) fn rcon_decode(ascii: &AsciiStr) -> RconResult<Team> {
        match ascii.as_str() {
            "0" => Ok(Team::Neutral),
            "1" => Ok(Team::One),
            "2" => Ok(Team::Two),
            _ => Err(RconError::protocol_msg(format!(
                "Unknown team Id {}",
                ascii
            ))),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq)]
pub enum Squad {
    NoSquad = 0,
    Alpha = 1,
    Bravo = 2,
    Charlie = 3,
    Delta = 4,
    Echo = 5,
    Foxtrot = 6,
    Golf = 7,
    Hotel = 8,
    India = 9,
    Juliet = 10,
    Kilo = 11,
    Lima = 12,
}

impl Squad {
    pub(crate) fn rcon_decode(words: &AsciiStr) -> RconResult<Self> {
        Ok(match words.as_str() {
            "0" => Squad::NoSquad,
            "1" => Squad::Alpha,
            "2" => Squad::Bravo,
            "3" => Squad::Charlie,
            "4" => Squad::Delta,
            "5" => Squad::Echo,
            "6" => Squad::Foxtrot,
            "7" => Squad::Golf,
            "8" => Squad::Hotel,
            "9" => Squad::India,
            "10" => Squad::Juliet,
            "11" => Squad::Kilo,
            "12" => Squad::Lima,
            _ => {
                return Err(RconError::protocol_msg(format!(
                    "Unknown squad Id {}",
                    words[0]
                )))
            }
        })
    }

    pub(crate) fn rcon_encode(&self) -> AsciiString {
        (*self as usize).to_string().into_ascii_string().unwrap()
    }
}

#[derive(Debug, Clone)]
pub enum Visibility {
    All,
    Team(Team),
    Squad(Team, Squad),
    Player(AsciiString),
}

impl Visibility {
    pub(crate) fn rcon_decode(words: &[AsciiString]) -> RconResult<(Self, usize)> {
        if words.is_empty() {
            return Err(RconError::protocol());
        }
        match words[0].as_str() {
            "all" => {
                if words.len() != 1 {
                    Err(RconError::protocol())
                } else {
                    Ok((Visibility::All, 1))
                }
            }
            "team" => {
                if words.len() != 2 {
                    Err(RconError::protocol())
                } else {
                    Ok((Visibility::Team(Team::rcon_decode(&words[1])?), 2))
                }
            }
            "squad" => {
                if words.len() != 3 {
                    Err(RconError::protocol())
                } else {
                    Ok((
                        Visibility::Squad(
                            Team::rcon_decode(&words[1])?,
                            Squad::rcon_decode(&words[2])?,
                        ),
                        3,
                    ))
                }
            }
            "player" => {
                if words.len() != 2 {
                    Err(RconError::protocol())
                } else {
                    Ok((Visibility::Player(words[1].clone()), 2))
                }
            }
            _ => Err(RconError::protocol()),
        }
    }

    pub(crate) fn rcon_encode(&self) -> Vec<AsciiString> {
        match self {
            Visibility::All => vec![AsciiString::from_str("all").unwrap()],
            Visibility::Team(team) => {
                vec![AsciiString::from_str("team").unwrap(), team.rcon_encode()]
            }
            Visibility::Squad(team, squad) => vec![
                AsciiString::from_str("squad").unwrap(),
                team.rcon_encode(),
                squad.rcon_encode(),
            ],
            Visibility::Player(player) => {
                vec![AsciiString::from_str("player").unwrap(), player.clone()]
            }
        }
    }
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
