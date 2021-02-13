use std::{convert::TryInto, str::FromStr};

use ascii::{AsciiChar, AsciiString};

pub enum EaidParseError {
    NotEaid,
}

/// EA GUID. Encoded as 32-long hex, without the EA_ prefix.
#[derive(Debug, Clone, Copy)]
pub struct Eaid ([AsciiChar; 32]);

impl Eaid {
    pub fn from_ascii(slice: &[AsciiChar]) -> Result<Eaid, EaidParseError> {
        if slice.len() == 32 + 3 {
            if slice[0..3] == ['E', 'A', '_'] {
                Ok(Eaid(slice.try_into().unwrap())) // we can use unwrap here because we tested the length
            } else {
                Err(EaidParseError::NotEaid)
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