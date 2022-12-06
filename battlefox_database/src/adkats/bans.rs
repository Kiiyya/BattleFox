//! Accessing tables related to player bans.

use sqlx::query;
use sqlx::types::time::OffsetDateTime;

use crate::BfoxContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BanStatus {
    Active,
    Expired,
    Disabled,
}

#[derive(Debug, Clone)]
pub struct BanInfo {
    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
    pub status: BanStatus,
    pub reason: String,
}

impl BfoxContext {
    /// Check whether the given player GUID is banned, and if yes, return the ban info.
    ///
    /// - `None` means there is no ban record for the player, i.e. not banned.
    /// - `Some((playerdata, ban))` means the player is probably banned, but please still
    ///    check `ban_status`, `end_time`, just to be sure.
    pub async fn get_ban(&self, guid: impl AsRef<str>) -> Result<Option<BanInfo>, sqlx::Error> {
        let ban = query!(
            "SELECT PlayerID, ClanTag, SoldierName, EAGUID, ban_notes, ban_status, ban_startTime, ban_endTime, record_message
            FROM tbl_playerdata AS pd
            INNER JOIN adkats_bans AS bans ON pd.PlayerId = bans.player_id
            INNER JOIN adkats_records_main AS records ON records.record_id = bans.latest_record_id
            WHERE pd.EAGUID = ?;"
        , guid.as_ref()).fetch_optional(&self.pool).await?;

        if let Some(ban) = ban {
            let status = match ban.ban_status.as_ref() {
                "Active" => BanStatus::Active,
                "Expired" => BanStatus::Expired,
                "Disabled" => BanStatus::Disabled,
                _ => unreachable!("Unknown ban status!")
            };

            let bi = BanInfo {
                start: ban.ban_startTime.assume_utc(),
                end: ban.ban_endTime.assume_utc(),
                status,
                reason: ban.record_message,
            };

            Ok(Some(bi))
        } else {
            Ok(None)
        }
    }
}
