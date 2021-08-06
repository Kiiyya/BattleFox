use crate::bf4::util::{parse_int, parse_bool};
use crate::rcon::{RconError, RconResult};
use ascii::AsciiString;
use serde::{Deserialize, Serialize};

///
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServerInfo {
    pub server_name: AsciiString,
    pub playercount: i32,
    pub max_playercount: i32,
    pub game_mode: AsciiString,
    pub map: AsciiString,
    pub rounds_played: i32,
    pub rounds_total: i32,
    pub scores: TeamScores,
    pub online_state: AsciiString,
    pub ranked: bool,
    pub punkbuster: bool,
    pub has_gamepassword: bool,
    pub server_uptime: i32,
    pub roundtime: i32,
    pub game_ip_and_port: AsciiString,
    pub punkbuster_version: AsciiString,
    pub join_queue_enabled: bool,
    pub region: AsciiString,
    pub closest_ping_site: AsciiString,
    pub country: AsciiString,
    pub blaze_player_count: i32,
    pub blaze_game_state: AsciiString,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamScores {
    pub number_of_entries: i32,
    pub scores: Vec<i32>,
    pub target_score: i32,
}

/// Expects the ServerInfo, without any leading "OK".
pub fn parse_serverinfo(words: &[AsciiString]) -> RconResult<ServerInfo> {
    if words.is_empty() {
        return Err(RconError::protocol_msg(
            "Failed to parse ServerInfo: Zero length?",
        ));
    }

    let teams_count = parse_int(&words[7])? as usize;
    let offset = teams_count;
    let server_info = ServerInfo {
        server_name: words[0].clone(),
        playercount: parse_int(&words[1])?,
        max_playercount: parse_int(&words[2])?,
        game_mode: words[3].clone(),
        map: words[4].clone(),
        rounds_played: parse_int(&words[5])?,
        rounds_total: parse_int(&words[6])?,
        scores: parse_teamscores(&words, teams_count),
        online_state: words[offset + 9].clone(),
        ranked: parse_bool(&words[offset + 10])?,
        punkbuster: parse_bool(&words[offset + 11])?,
        has_gamepassword: parse_bool(&words[offset + 12])?,
        server_uptime: parse_int(&words[offset + 13])?,
        roundtime: parse_int(&words[offset + 14])?,
        game_ip_and_port: words[offset + 15].clone(),
        punkbuster_version: words[offset + 16].clone(),
        join_queue_enabled: parse_bool(&words[offset + 17])?,
        region: words[offset + 18].clone(),
        closest_ping_site: words[offset + 19].clone(),
        country: words[offset + 20].clone(),
        blaze_player_count: parse_int(&words[offset + 21])?,
        blaze_game_state: words[offset + 22].clone(),
    };

    Ok(server_info)
}

fn parse_teamscores(words: &[AsciiString], teams_count: usize) -> TeamScores {
    let mut offset = 8;
    let mut teamscores: Vec<i32> = Vec::new();
    for _ in 0..teams_count {
        teamscores.push(parse_int(&words[offset]).unwrap());

        offset += 1;
    }

    TeamScores {
        number_of_entries: teams_count as i32,
        scores: teamscores,
        target_score: parse_int(&words[offset]).unwrap(),
    }
}
