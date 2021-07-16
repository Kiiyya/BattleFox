use ascii::AsciiString;

use crate::rcon::{RconError, RconResult};

pub fn parse_int(word: &AsciiString) -> RconResult<i32> {
    word.as_str().parse::<i32>().map_err(|_| {
        RconError::protocol_msg(format!(
            "Failed to parse: \"{}\" is not an unsigned integer",
            word
        ))
    })
}

pub fn parse_bool(word: &AsciiString) -> RconResult<bool> {
    word.as_str().parse::<bool>().map_err(|_| {
        RconError::protocol_msg(format!(
            "Failed to parse: \"{}\" is not boolean",
            word
        ))
    })
}

