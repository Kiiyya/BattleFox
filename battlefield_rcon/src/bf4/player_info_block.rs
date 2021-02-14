
use ascii::AsciiString;
use super::{ea_guid::{Eaid, EaidParseError}, visibility::{Squad, Team}};

pub enum ParsePibError {
    Derp,
}

impl From<EaidParseError> for ParsePibError {
    fn from(_: EaidParseError) -> Self {
        ParsePibError::Derp
    }
}

/// One row in the PlayerInfoBlock.
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

fn parse_int(word: &AsciiString) -> Result<usize, ParsePibError> {
    word.as_str().parse::<usize>().map_err(|_| ParsePibError::Derp)
}

fn assert_len(words: &[AsciiString], len: usize) -> Result<(), ParsePibError> {
    if words.len() != len {
        Err(ParsePibError::Derp)
    } else {
        Ok(())
    }
}

/// Expects the PIB, without any leading "OK" though.
pub fn parse_pib(words: &[AsciiString]) -> Result<Vec<PlayerInfo>, ParsePibError> {
    if words.len() == 0 {
        return Err(ParsePibError::Derp);
    }

    // word offset.
    let mut offset = 0;

    let n_columns = parse_int(&words[offset])?;
    if words.len() - offset < n_columns || n_columns != 9 { // currently there are 9 columns
        return Err(ParsePibError::Derp);
    }
    offset += 1;

    // now read in the column names, make sure we're still talking about the same thing
    const COLS : [&'static str; 9] = ["name", "guid", "teamId", "squadId", "kills", "deaths", "score", "rank", "ping"];
    for i in 0..9 {
        let col_name = words[offset + i].as_str();
        if col_name != COLS[i] {
            return Err(ParsePibError::Derp);
        }
    }
    offset += 9;

    // now read in how many rows (= players) we have.
    if words.len() - offset == 0 {
        return Err(ParsePibError::Derp);
    }
    let m_rows = parse_int(&words[offset])?;
    offset += 1;

    // make sure there actually is enough words to read in, that that packet isn't malformed.
    if words.len() - offset != n_columns * m_rows {
        return Err(ParsePibError::Derp);
    }

    // now we actually read in the data.
    let pib = Vec::new();
    for m in 0..m_rows {
        let pi = PlayerInfo {
            player_name: words[offset + 0],
            eaid: Eaid::from_ascii(&words[offset + 1])?,
            team: Team::from_rcon_format(&words[offset + 2])?,
            squad: Squad::from_rcon_format(&words[offset + 3]),
            kills: (),
            deahts: (),
            score: (),
            rank: (),
            ping: (),
        };

        offset += 9;
    }


    Ok(pib)
}
