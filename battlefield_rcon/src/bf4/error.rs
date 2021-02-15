use ascii::AsciiString;

use crate::rcon::RconError;

#[derive(Debug, Clone)]
pub enum Bf4Error {
    PlayerGuidResolveFailed {
        player_name: AsciiString,
        rcon: Option<RconError>,
    },
    UnknownEvent(Vec<AsciiString>),
    Rcon(RconError),
    Other(String),
}

impl Bf4Error {
    pub fn other(str: impl Into<String>) -> Self {
        Self::Other(str.into())
    }
}

impl From<RconError> for Bf4Error {
    fn from(e: RconError) -> Self {
        Self::Rcon(e)
    }
}

// impl From<ParsePacketError> for Bf4Error {
//     fn from(e: ParsePacketError) -> Self {
//         Self::other("Failed to parse packet")
//     }
// }

pub type Bf4Result<T> = Result<T, Bf4Error>;
