use std::str::FromStr;

use ascii::{AsciiStr, AsciiString, IntoAsciiString};
use serde::{Deserialize, Serialize};

use crate::{rcon::RconError, rcon::RconResult};

use super::Player;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    Mike = 13,
    November = 14,
    Oscar = 15,
    Papa = 16,
    Quebec = 17,
    Romeo = 18,
    Sierra = 19,
    Tango = 20,
    Uniform = 21,
    Victor = 22,
    Whiskey = 23,
}

impl Squad {
    pub(crate) fn rcon_decode(word: &AsciiStr) -> RconResult<Self> {
        Ok(match word.as_str() {
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
            "13" => Squad::Mike,
            "14" => Squad::November,
            "15" => Squad::Oscar,
            "16" => Squad::Papa,
            "17" => Squad::Quebec,
            "18" => Squad::Romeo,
            "19" => Squad::Sierra,
            "20" => Squad::Tango,
            "21" => Squad::Uniform,
            "22" => Squad::Victor,
            "23" => Squad::Whiskey,
            _ => {
                return Err(RconError::protocol_msg(format!(
                    "Unknown squad Id {}",
                    word
                )))
            }
        })
    }

    pub(crate) fn rcon_encode(&self) -> AsciiString {
        (*self as usize).to_string().into_ascii_string().unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl From<&Player> for Visibility {
    fn from(p: &Player) -> Self {
        Visibility::Player(p.name.clone())
    }
}

impl From<Player> for Visibility {
    fn from(p: Player) -> Self {
        Visibility::Player(p.name)
    }
}
