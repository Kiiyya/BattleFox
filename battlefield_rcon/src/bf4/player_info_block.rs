use crate::bf4::util::parse_int;
use crate::rcon::{RconError, RconResult};
use ascii::AsciiString;
use serde::{Deserialize, Serialize};
use crate::bf4::RconDecoding;

use super::{
    ea_guid::Eaid,
    Squad, Team,
};

// pub enum ParsePibError {
//     Derp,
// }

// impl From<EaidParseError> for ParsePibError {
//     fn from(_: EaidParseError) -> Self {
//         ParsePibError::Derp
//     }
// }

/// One row in the PlayerInfoBlock.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub player_name: AsciiString,
    pub eaid: Eaid,
    pub squad: Squad,
    pub team: Team,
    pub kills: i32,
    pub deaths: i32,
    pub score: i32,
    pub rank: i32,
    pub ping: i32,
}

// fn assert_len(words: &[AsciiString], len: usize) -> Bf4Result<()> {
//     if words.len() != len {
//         Err(Bf4Error::Rcon(RconError::protocol()))
//     } else {
//         Ok(())
//     }
// }

/// Expects the PIB, without any leading "OK" though.
pub fn parse_pib(words: &[AsciiString]) -> RconResult<Vec<PlayerInfo>> {
    if words.is_empty() {
        return Err(RconError::protocol_msg(
            "Failed to parse PlayerInfoBlock: Zero length?",
        ));
    }

    // word offset.
    let mut offset = 0;

    const N_COLS: usize = 10;
    let n_columns = parse_int(&words[offset])? as usize;
    if words.len() - offset < n_columns || n_columns != N_COLS {
        // currently there are 9 columns
        return Err(RconError::protocol_msg(format!(
            "Failed to parse PlayerInfoBlock: Expected {} columns",
            N_COLS
        )));
    }
    offset += 1;

    // now read in the column names, make sure we're still talking about the same thing
    const COLS: [&str; N_COLS] = [
        "name", "guid", "teamId", "squadId", "kills", "deaths", "score", "rank", "ping", "type",
    ];
    for i in 0..N_COLS {
        let col_name = words[offset + i].as_str();
        if col_name != COLS[i] {
            return Err(RconError::protocol_msg(
                "Failed to parse PlayerInfoBlock: Column mismatch, did the rcon protocol change?",
            ));
        }
    }
    offset += N_COLS;

    // now read in how many rows (= players) we have.
    if words.len() - offset == 0 {
        return Err(RconError::protocol_msg("Failed to parse PlayerInfoBlock"));
    }
    let m_rows = parse_int(&words[offset])? as usize;
    offset += 1;

    // make sure there actually is enough words to read in, that that packet isn't malformed.
    if words.len() - offset != n_columns * m_rows {
        return Err(RconError::protocol_msg("Failed to parse PlayerInfoBlock"));
    }

    // now we actually read in the data.
    let mut pib = Vec::new();
    for _ in 0..m_rows {
        let pi = PlayerInfo {
            player_name: words[offset].clone(),
            eaid: Eaid::rcon_decode(&words[offset + 1])?,
            team: Team::rcon_decode(&words[offset + 2])?,
            squad: Squad::rcon_decode(&words[offset + 3])?,
            kills: parse_int(&words[offset + 4])?,
            deaths: parse_int(&words[offset + 5])?,
            score: parse_int(&words[offset + 6])?,
            rank: parse_int(&words[offset + 7])?,
            ping: parse_int(&words[offset + 8])?,
        };

        pib.push(pi);

        offset += N_COLS;
    }

    Ok(pib)
}
