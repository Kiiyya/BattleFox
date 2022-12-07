use crate::bf4::util::{parse_int};
use crate::rcon::{RconError, RconResult};
use ascii::AsciiString;
use serde::{Deserialize, Serialize};

///
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamScores {
    pub number_of_entries: i32,
    pub scores: Vec<i32>,
    pub target_score: i32,
}

/// Expects the TeamScores, without any leading "OK".
pub fn parse_team_scores(words: &[AsciiString]) -> RconResult<TeamScores> {
    if words.is_empty() {
        return Err(RconError::protocol_msg(
            "Failed to parse TeamScores: Zero length?",
        ));
    }

    let teams_count = parse_int(&words[1])? as usize;
    let mut offset = 2;
    let mut teamscores: Vec<i32> = Vec::new();
    for _ in 0..teams_count {
        teamscores.push(parse_int(&words[offset]).unwrap());

        offset += 1;
    }

    Ok(TeamScores {
        number_of_entries: teams_count as i32,
        scores: teamscores,
        target_score: parse_int(&words[offset]).unwrap(),
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    use ascii::{AsciiString, IntoAsciiString};

    #[test]
    fn parse_zero_teams() {
        let mut words: Vec<AsciiString> = Vec::new();
        words.push("server.onRoundOverTeamScores".into_ascii_string().unwrap());
        words.push("0".into_ascii_string().unwrap());
        words.push("0".into_ascii_string().unwrap());
        let result = parse_team_scores(&words).unwrap();

        assert_eq!(0, result.number_of_entries);
        assert_eq!(0, result.scores.len());
        assert_eq!(0, result.target_score);
    }

    #[test]
    fn parse_two_teams() {
        let mut words: Vec<AsciiString> = Vec::new();
        words.push("server.onRoundOverTeamScores".into_ascii_string().unwrap());
        words.push("2".into_ascii_string().unwrap());
        words.push("100".into_ascii_string().unwrap());
        words.push("110".into_ascii_string().unwrap());
        words.push("150".into_ascii_string().unwrap());
        let result = parse_team_scores(&words).unwrap();

        assert_eq!(2, result.number_of_entries);
        assert_eq!(2, result.scores.len());
        assert_eq!(100, result.scores[0]);
        assert_eq!(110, result.scores[1]);
        assert_eq!(150, result.target_score);
    }
}
