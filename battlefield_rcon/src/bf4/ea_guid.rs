use std::{convert::TryInto, str::FromStr};

use ascii::{AsAsciiStr, AsciiChar, AsciiString};

#[derive(Debug, Clone, Copy)]
pub enum EaidParseError {
    NotEaid,
}

/// EA GUID. Encoded as 32-long hex, without the EA_ prefix.
#[derive(Debug, Clone, Copy)]
pub struct Eaid ([AsciiChar; 32]);

impl Eaid {
    pub fn from_ascii(ascii: &AsciiString) -> Result<Eaid, EaidParseError> {
        let str = ascii.as_str();
        if str.len() == 32 + 3 {
            if &str[0..3] != "EA_" {
                Err(EaidParseError::NotEaid)
            }
            else {
                Ok(Eaid(str.as_ascii_str().unwrap().try_into().unwrap())) // we can use unwrap here because we tested the length
            }
        } else {
            Err(EaidParseError::NotEaid)
        }
    }

    /// Returns stuff like "EA_FFFF...FFF". Always 32+3 length.
    pub fn to_ascii(&self) -> AsciiString {
        let mut ascii = AsciiString::from_str("EA_").unwrap();
        for &c in self.0.iter() {
            ascii.push(c);
        }
        ascii
    }
}
