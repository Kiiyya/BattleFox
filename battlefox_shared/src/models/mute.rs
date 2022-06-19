use chrono::NaiveDate;
use std::convert::TryFrom;

#[derive(Debug)]
pub struct MutedPlayer {
    pub eaid: String,
    pub r#type: MuteType,
    pub end_date: Option<NaiveDate>,
    pub kicks: Option<u32>
}

#[derive(Debug, PartialEq, Eq)]
pub enum MuteType {
    Disabled = 0,
    Round = 1,
    Days = 2,
    Permanent = 3
}

impl TryFrom<i32> for MuteType {
    type Error = ();

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            x if x == MuteType::Disabled as i32 => Ok(MuteType::Disabled),
            x if x == MuteType::Round as i32 => Ok(MuteType::Round),
            x if x == MuteType::Days as i32 => Ok(MuteType::Days),
            x if x == MuteType::Permanent as i32 => Ok(MuteType::Permanent),
            _ => Err(()),
        }
    }
}