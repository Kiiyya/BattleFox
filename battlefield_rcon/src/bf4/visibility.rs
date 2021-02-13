use ascii::{AsciiChar, AsciiString};

use super::ParsePacketError;


#[derive(Debug, Copy, Clone)]
pub enum Team {
    Neutral = 0,
    One = 1,
    Two = 2,
}

impl Team {
    pub fn to_rcon_format(self) -> String {
        (self as usize).to_string()
    }

    pub fn from_rcon_format<'a>(ascii: &AsciiString) -> Result<Team, ParsePacketError> {
        match ascii.as_str() {
            "0" => Ok(Team::Neutral),
            "1" => Ok(Team::One),
            "2" => Ok(Team::Two),
            _   => Err(ParsePacketError::InvalidVisibility),
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
    /// Returns "2" for Bravo, 0 for "NoSquad", ...
    pub fn rcon_format(self) -> String {
        (self as usize).to_string()
    }

    pub fn from_rcon_format(ascii: &AsciiString) -> Result<Self, ParsePacketError> {
        match ascii.as_str() {
            "0" => Ok(Squad::NoSquad),
            "1" => Ok(Squad::Alpha),
            "2" => Ok(Squad::Bravo),
            "3" => Ok(Squad::Charlie),
            "4" => Ok(Squad::Delta),
            "5" => Ok(Squad::Echo),
            "6" => Ok(Squad::Foxtrot),
            "7" => Ok(Squad::Golf),
            "8" => Ok(Squad::Hotel),
            "9" => Ok(Squad::India),
            "10" => Ok(Squad::Juliet),
            "11" => Ok(Squad::Kilo),
            "12" => Ok(Squad::Lima),
            _   => Err(ParsePacketError::InvalidVisibility),
        }
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
    pub fn to_rcon_format(&self) -> String {
        match self {
            Visibility::All => "all".into(),
            Visibility::Team(team) => format!("team {}", team.to_rcon_format()),
            Visibility::Squad(team, squad) => format!("squad {} {}", team.to_rcon_format(), squad.rcon_format()),
            Visibility::Player(player) => format!("player {}", player),
        }
    }

    pub fn from_rcon_format(str: &AsciiString) -> Result<Self, ParsePacketError> {
        let split : Vec<_> = str.split(AsciiChar::Space).collect::<Vec<_>>();
        if split.len() == 0 {
            return Err(ParsePacketError::InvalidVisibility);
        }
        match split[0].as_str() {
            "all" => {
                if split.len() != 1 {
                    return Err(ParsePacketError::InvalidVisibility);
                }
                Ok(Visibility::All)
            },
            "team" => {
                if split.len() != 2 {
                    return Err(ParsePacketError::InvalidVisibility);
                }
                Ok(Visibility::Team(Team::from_rcon_format(&split[1].into())?))
            },
            "squad" => {
                if split.len() != 3 {
                    return Err(ParsePacketError::InvalidVisibility);
                }
                Ok(Visibility::Squad(
                    Team::from_rcon_format(&split[1].into())?,
                    Squad::from_rcon_format(&split[2].into())?
                ))
            },
            "player" => {
                if split.len() != 2 {
                    return Err(ParsePacketError::InvalidVisibility);
                }
                Ok(Visibility::Player(
                    split[1].into()
                ))
            }
            _ => Err(ParsePacketError::InvalidVisibility),
        }
    }
}
