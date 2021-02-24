use std::{convert::TryInto, fmt::Display, str::FromStr};

use ascii::{AsAsciiStr, AsciiChar, AsciiStr, AsciiString};

#[derive(Debug, Clone)]
pub struct EaidParseError;

/// EA GUID. Encoded as 32-long hex, without the EA_ prefix.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Eaid([AsciiChar; 32]);

// pub const EAID_SERVER : Eaid = Eaid([AsciiChar::_0; 32]);

impl Eaid {
    /// Input: "EA_FFFF..."
    pub fn from_rcon_format(ascii: &AsciiStr) -> Result<Eaid, EaidParseError> {
        let str = ascii.as_str();
        if str.len() == 32 + 3 {
            if &str[0..3] != "EA_" {
                Err(EaidParseError)
            } else {
                let guid_only = &ascii.as_slice()[3..]; // skip "EA_"
                Ok(Eaid(guid_only.try_into().unwrap())) // we can use unwrap here because we tested the length
            }
        } else if str.is_empty() {
            Ok(Eaid([AsciiChar::X; 32]))
        } else {
            Err(EaidParseError)
        }
    }

    /// Returns stuff like "EA_FFFF...FFF". Always 32+3 length.
    pub fn to_ascii(&self) -> AsciiString {
        let mut ascii = AsciiString::from_str("EA_").unwrap();
        for &char in self.0.iter() {
            ascii.push(char);
        }
        ascii
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
