use ascii::AsciiString;
use std::{
    convert::TryInto,
    fmt::{Display, Formatter},
    usize, write,
};

#[derive(PartialEq, Eq, Debug)]
pub enum PacketOrigin {
    Client,
    Server,
}

impl Display for PacketOrigin {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PacketOrigin::Client => write!(f, "Client"),
            PacketOrigin::Server => write!(f, "Server"),
        }
    }
}

#[derive(Debug)]
pub struct Packet {
    pub sequence: u32,
    pub origin: PacketOrigin,
    pub is_response: bool,
    pub words: Vec<AsciiString>,
}

impl Display for Packet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}#{} origin={}:",
            if self.is_response { 'r' } else { ' ' },
            self.sequence,
            self.origin,
            // self.words
        )?;
        for w in &self.words {
            write!(f, " {}", w)?;
        }
        Ok(())
    }
}

pub enum PacketDeserializeResult {
    Ok {
        packet: Packet,
        consumed_bytes: usize,
    },
    BadHeader,
    BufferTooSmall {
        packet_size: usize,
    },
    Malformed,
}

impl Packet {
    pub fn serialize(&self) -> Vec<u8> {
        let words_len = self
            .words
            .iter()
            .fold(0, |accu, word| accu + word.len() + 4 + 1); // +4 because word len as u32, +1 because '\0'.
        let total_len = words_len + 12;

        let mut buf = vec![0_u8; total_len];

        let mut seq_id = self.sequence & 0x3fffffff;
        if self.origin == PacketOrigin::Server {
            seq_id |= 0x80000000
        };
        if self.is_response {
            seq_id |= 0x40000000
        };

        // first, header.

        buf[0..4].copy_from_slice(&seq_id.to_le_bytes());
        // total packet size
        buf[4..8].copy_from_slice(&(total_len as u32).to_le_bytes());
        // amoutn of words
        buf[8..12].copy_from_slice(&(self.words.len() as u32).to_le_bytes());

        // then, payload (just a bunch of strings (aka words))

        let mut offset = 12;
        for word in self.words.iter() {
            // length of current word as u32
            buf[offset..offset + 4].copy_from_slice(&(word.len() as u32).to_le_bytes());
            offset += 4;
            // word itself
            buf[offset..offset + word.len()].copy_from_slice(word.as_bytes());
            offset += word.len();

            buf[offset] = 0; // null terminator
            offset += 1;
        }
        buf
    }

    /// reads total lentgh from packet header. Including the header though!
    /// So you need to subtract 12 bytes potentially.
    pub fn read_total_len(buf: &[u8; 12]) -> usize {
        let total_len = u32::from_le_bytes(buf[4..8].try_into().unwrap()) as usize;
        total_len
    }

    pub fn deserialize(buf: &[u8]) -> PacketDeserializeResult {
        if buf.len() < 12 {
            return PacketDeserializeResult::BadHeader;
        }
        let first = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        let seq_id = first & 0x3fffffff;
        let is_response = (first & 0x40000000) != 0;
        let origin = if (first & 0x80000000) == 0 {
            PacketOrigin::Server
        } else {
            PacketOrigin::Client
        };

        let total_len = u32::from_le_bytes(buf[4..8].try_into().unwrap()) as usize;
        let word_count = u32::from_le_bytes(buf[8..12].try_into().unwrap()) as usize;
        if buf.len() < total_len {
            return PacketDeserializeResult::BufferTooSmall {
                packet_size: total_len,
            };
        }

        let mut offset = 12;
        let mut words = Vec::with_capacity(word_count);
        for _ in 0..word_count {
            if offset + 4 > total_len {
                return PacketDeserializeResult::Malformed;
            }
            let len = u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap()) as usize;

            offset += 4;
            if offset + len > total_len {
                return PacketDeserializeResult::Malformed;
            }
            let wordbytes: Vec<u8> = buf[offset..offset + len].try_into().unwrap();
            match AsciiString::from_ascii(wordbytes) {
                Ok(str) => {
                    words.push(str);
                    offset += len;
                    offset += 1; // null byte which we just ignore.
                }
                Err(_) => return PacketDeserializeResult::Malformed,
            }
        }

        if offset != total_len {
            return PacketDeserializeResult::Malformed;
        }
        PacketDeserializeResult::Ok {
            packet: Packet {
                sequence: seq_id,
                is_response,
                origin,
                words,
            },
            consumed_bytes: total_len,
        }
    }
}
