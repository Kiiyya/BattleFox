use std::{convert::TryInto, fmt::Display, str::FromStr};

use ascii::{AsAsciiStr, AsciiChar, AsciiStr, AsciiString};
use serde::{Deserialize, Serialize, de::{Error, Visitor}};

use crate::rcon::RconError;

use super::{RconDecoding, RconEncoding};

#[derive(Debug, Clone)]
pub struct EaidParseError;

/// EA GUID. Encoded as 32-long hex, without the EA_ prefix.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Eaid([AsciiChar; 32]);

impl Serialize for Eaid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        serializer.serialize_str(self.rcon_encode().as_str())
    }
}

impl<'de> Deserialize<'de> for Eaid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
            deserializer.deserialize_str(EaidVisitor)
    }
}

struct EaidVisitor;

impl <'de> Visitor<'de> for EaidVisitor {
    type Value = Eaid;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("an EA GUID in form of EA_0123456789ABCDEF...F")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match AsciiStr::from_ascii(v) {
            Ok(ascii) => {
                match Eaid::rcon_decode(ascii) {
                    Ok(eaid) => Ok(eaid),
                    Err(_) => Err(E::custom(format!("Not a valid EA GUID: {}", v))),
                }
            }
            Err(_) => Err(E::custom(format!("Not valid ASCII when parsing EA GUID: {}", v))),
        }
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
            E: Error, {
        self.visit_str(v.as_str())
    }
}

impl RconDecoding for Eaid {
    fn rcon_decode(ascii: &AsciiStr) -> crate::rcon::RconResult<Self> {
        let str = ascii.as_str();
        if str.len() == 32 + 3 {
            if &str[0..3] != "EA_" {
                Err(RconError::protocol_msg(format!("Trying to decode \"{}\" into an EA GUID failed", ascii)))
            } else {
                let guid_only = &ascii.as_slice()[3..]; // skip "EA_"
                Ok(Eaid(guid_only.try_into().unwrap())) // we can use unwrap here because we tested the length
            }
        } else if str.is_empty() {
            Ok(Eaid([AsciiChar::X; 32]))
        } else {
            Err(RconError::protocol_msg(format!("Trying to decode \"{}\" into an EA GUID failed", ascii)))
        }
    }
}

impl RconEncoding for Eaid {
    /// Returns stuff like "EA_FFFF...FFF". Always 32+3 length.
    fn rcon_encode(&self) -> AsciiString {
        let mut ascii = AsciiString::from_str("EA_").unwrap();
        for &char in self.0.iter() {
            ascii.push(char);
        }
        ascii
    }
}

impl Eaid {
    pub fn new(ascii: &AsciiString) -> crate::rcon::RconResult<Self> {
        let str = ascii.as_str();
        if str.len() == 32 + 3 {
            if &str[0..3] != "EA_" {
                Err(RconError::protocol_msg(format!("Trying to decode \"{}\" into an EA GUID failed", ascii)))
            } else {
                let guid_only = &ascii.as_slice()[3..]; // skip "EA_"
                Ok(Eaid(guid_only.try_into().unwrap())) // we can use unwrap here because we tested the length
            }
        } else if str.is_empty() {
            Ok(Eaid([AsciiChar::X; 32]))
        } else {
            Err(RconError::protocol_msg(format!("Trying to decode \"{}\" into an EA GUID failed", ascii)))
        }
    }
}

impl Display for Eaid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EA_{}", self.0.as_ascii_str().unwrap())
    }
}

impl std::fmt::Debug for Eaid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EA_{}", self.0.as_ascii_str().unwrap())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::bf4::Player;


    #[test]
    fn serialization() {
        let eaid = Eaid([AsciiChar::F; 32]);
        let player = Player {
            name: AsciiString::new(),
            eaid,
        };
        let ser = serde_json::to_string(&player).unwrap();
        assert_eq!(r#"{"name":"","eaid":"EA_FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"}"#, ser);
        let de : Player = serde_json::from_str(ser.as_str()).unwrap();
        assert_eq!(de, player);
    }
}