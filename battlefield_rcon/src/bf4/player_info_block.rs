
use ascii::AsciiString;
use crate::rcon::{RconError, RconResult};

use super::{ea_guid::{Eaid, EaidParseError}, visibility::{Squad, Team}};

// pub enum ParsePibError {
//     Derp,
// }

// impl From<EaidParseError> for ParsePibError {
//     fn from(_: EaidParseError) -> Self {
//         ParsePibError::Derp
//     }
// }

/// One row in the PlayerInfoBlock.
#[derive(Debug, Clone)]
pub struct PlayerInfo {
    pub player_name: AsciiString,
    pub eaid: Eaid,
    pub squad: Squad,
    pub team: Team,
    pub kills: usize,
    pub deahts: usize,
    pub score: usize,
    pub rank: usize,
    pub ping: usize,
}

fn parse_int(word: &AsciiString) -> RconResult<usize> {
    word.as_str().parse::<usize>().map_err(|_| RconError::protocol_msg(format!("Failed to parse PlayerInfoBlock: \"{}\" is not an unsigned integer", word)))
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
    if words.len() == 0 {
        return Err(RconError::protocol_msg("Failed to parse PlayerInfoBlock: Zero length?"));
    }

    // word offset.
    let mut offset = 0;

    let n_columns = parse_int(&words[offset])?;
    if words.len() - offset < n_columns || n_columns != 9 { // currently there are 9 columns
        return Err(RconError::protocol_msg(format!("Failed to parse PlayerInfoBlock: Expected 9 columns")));
    }
    offset += 1;

    // now read in the column names, make sure we're still talking about the same thing
    const COLS : [&'static str; 9] = ["name", "guid", "teamId", "squadId", "kills", "deaths", "score", "rank", "ping"];
    for i in 0..9 {
        let col_name = words[offset + i].as_str();
        if col_name != COLS[i] {
            return Err(RconError::protocol_msg(format!("Failed to parse PlayerInfoBlock: Column mismatch, did the rcon protocol change?")));
        }
    }
    offset += 9;

    // now read in how many rows (= players) we have.
    if words.len() - offset == 0 {
        return Err(RconError::protocol_msg(format!("Failed to parse PlayerInfoBlock")));
    }
    let m_rows = parse_int(&words[offset])?;
    offset += 1;

    // make sure there actually is enough words to read in, that that packet isn't malformed.
    if words.len() - offset != n_columns * m_rows {
        return Err(RconError::protocol_msg(format!("Failed to parse PlayerInfoBlock")));
    }

    // now we actually read in the data.
    let mut pib = Vec::new();
    for _ in 0..m_rows {
        let pi = PlayerInfo {
            player_name: words[offset + 0].clone(),
            eaid: Eaid::from_rcon_format(&words[offset + 1]).map_err(|_:EaidParseError| RconError::protocol_msg("Failed to parse PlayerInfoBlock: Invalid EA GUID"))?,
            team: Team::from_rcon_format(&words[offset + 2])?,
            squad: Squad::from_rcon_format(&words[offset + 3])?,
            kills: parse_int(&words[offset + 4])?,
            deahts: parse_int(&words[offset + 5])?,
            score: parse_int(&words[offset + 6])?,
            rank: parse_int(&words[offset + 7])?,
            ping: parse_int(&words[offset + 8])?,
        };

        pib.push(pi);

        offset += 9;
    }

    Ok(pib)
}
