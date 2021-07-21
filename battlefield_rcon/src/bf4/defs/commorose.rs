use crate::{rcon::RconError, rcon::RconResult};

use ascii::AsciiStr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommmoRose {
    AttackDefend,
    Thanks,
    Sorry,
    GoGoGo,
    RequestOrder,
    RequestMedic,
    RequestAmmo,
    RequestRide,
    GetOut,
    GetIn,
    RequestRepairs,
    Affirmative,
    Negative
}

impl CommmoRose {
    pub fn decode(word: &AsciiStr) -> RconResult<Self> {
        if word.is_empty() || !word.as_str().starts_with("ID_CHAT_") {
            return Err(RconError::protocol());
        }
        Ok(match word.as_str() {
            "ID_CHAT_ATTACK/DEFEND" => CommmoRose::AttackDefend,
            "ID_CHAT_THANKS" => CommmoRose::Thanks,
            "ID_CHAT_SORRY" => CommmoRose::Sorry,
            "ID_CHAT_GOGOGO" => CommmoRose::GoGoGo,
            "ID_CHAT_REQUEST_ORDER" => CommmoRose::RequestOrder,
            "ID_CHAT_REQUEST_MEDIC" => CommmoRose::RequestMedic,
            "ID_CHAT_REQUEST_AMMO" => CommmoRose::RequestAmmo,
            "ID_CHAT_REQUEST_RIDE" => CommmoRose::RequestRide,
            "ID_CHAT_GET_OUT" => CommmoRose::GetOut,
            "ID_CHAT_GET_IN" => CommmoRose::GetIn,
            "ID_CHAT_REQUEST_REPAIRS" => CommmoRose::RequestRepairs,
            "ID_CHAT_AFFIRMATIVE" => CommmoRose::Affirmative,
            "ID_CHAT_NEGATIVE" => CommmoRose::Negative,
            _ => {
                return Err(RconError::protocol_msg(format!(
                    "Unknown Commo Rose message {}",
                    word
                )))
            }
        })
    }

    #[allow(non_snake_case)]
    pub fn pretty(&self) -> &'static str {
        match self {
            CommmoRose::AttackDefend => "ATTACK / DEFEND",
            CommmoRose::Thanks => "THANKS",
            CommmoRose::Sorry => "SORRY",
            CommmoRose::GoGoGo => "GO GO GO",
            CommmoRose::RequestOrder => "REQUEST ORDER",
            CommmoRose::RequestMedic => "REQUEST MEDIC",
            CommmoRose::RequestAmmo => "REQUEST AMMO",
            CommmoRose::RequestRide => "REQUEST RIDE",
            CommmoRose::GetOut => "GET OUT",
            CommmoRose::GetIn => "GET IN",
            CommmoRose::RequestRepairs => "REQUEST REPAIRS",
            CommmoRose::Affirmative => "AFFIRMATIVE",
            CommmoRose::Negative => "NEGATIVE",
        }
    }
}