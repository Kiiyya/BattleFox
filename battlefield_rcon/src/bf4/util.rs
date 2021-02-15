use ascii::AsciiString;

use crate::rcon::{RconError, RconResult};

pub fn parse_int(word: &AsciiString) -> RconResult<i32> {
    word.as_str().parse::<i32>().map_err(|_| {
        RconError::protocol_msg(format!(
            "Failed to parse PlayerInfoBlock: \"{}\" is not an unsigned integer",
            word
        ))
    })
}
