//! Utilities for parsing the Bf4 RCON "Map List" Packet type.
//! Has nothing to do with the MapList extension/plugin/whatchamacallit of BattleFox.
use ascii::AsciiString;

use crate::rcon::{RconError, RconResult};

use super::{util::parse_int, GameMode, Map, RconDecoding};

#[derive(Debug, Clone)]
pub struct MapListEntry {
    pub map: Map,
    pub game_mode: GameMode,
    pub n_rounds: usize,
}

pub fn parse_map_list(words: &[AsciiString]) -> RconResult<Vec<MapListEntry>> {
    if words.len() < 2 {
        return Err(RconError::protocol_msg(
            "Failed to parse MapList: Zero length?",
        ));
    }

    // first, header (Two words).
    let mut offset = 0;
    let n_maps = parse_int(&words[0])? as usize;
    let words_per_map = parse_int(&words[1])? as usize;
    offset += 2;

    if words_per_map != 3 {
        return Err(RconError::protocol_msg(
            format!("Failed to parse MapList: Did the RCON protocol change? Expected 3 words per map, but found {}", words_per_map),
        ));
    }

    if words.len() - offset != n_maps * words_per_map {
        return Err(RconError::protocol_msg(format!(
            "Failed to parse MapList: {} words expected, but only found {}",
            n_maps * words_per_map,
            words.len() - offset
        )));
    }
    let mut ret = Vec::with_capacity(n_maps as usize);

    for _ in 0..n_maps {
        ret.push(MapListEntry {
            map: Map::rcon_decode(&words[offset])?,
            game_mode: GameMode::rcon_decode(&words[offset + 1])?,
            n_rounds: parse_int(&words[offset + 2])? as usize,
        });

        offset += words_per_map;
    }

    Ok(ret)
}
