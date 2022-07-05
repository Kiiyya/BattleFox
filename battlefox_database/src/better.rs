//! A new, better, attempt at battlefox_database with SQLx. Because Diesel is clunky.
//!
//! And yet I'm too lazy to get rid of diesel, so that's why I'm just making a `better` module
//! instead of rewriting the whole thing.

// pub struct Config { }

use sqlx::mysql::MySqlPoolOptions;
pub use sqlx::types::time::OffsetDateTime; // apparently sqlX uses its own datetime stuff.
use thiserror::Error;
use sqlx::query;

/// Database Connection Pool. Cheap to clone, inner reference counting.
#[derive(Clone)]
pub struct BfoxDb {
    db: sqlx::MySqlPool,
}

#[derive(Error, Debug)]
pub enum BfoxDbError {
    #[error("{0}")]
    DbErr(#[from] sqlx::Error),
}

impl BfoxDb {
    /// Connect to database.
    ///
    /// `db_uri` is something like `protocol://username:password@host/database`.
    ///
    /// https://www.sea-ql.org/SeaORM/docs/install-and-config/connection
    pub async fn new(db_uri: impl AsRef<str>) -> Result<Self, BfoxDbError> {
        // let db: DatabaseConnection = Database::connect(db_uri.as_ref()).await?;
        let db = MySqlPoolOptions::new()
            // .max_connections(3)
            .connect(db_uri.as_ref())
            .await?;

        Ok(Self {
            db
        })
    }

    /// Check whether the given player GUID is banned, and if yes, return the ban info.
    ///
    /// - `None` means there is no ban record for the player, i.e. not banned.
    /// - `Some((playerdata, ban))` means the player is probably banned, but please still
    ///    check `ban_status`, `end_time`, just to be sure.
    pub async fn get_ban(&self, guid: impl AsRef<str>) -> Result<Option<BanInfo>, BfoxDbError> {
        let ban = query!(
            "SELECT PlayerID, ClanTag, SoldierName, EAGUID, ban_notes, ban_status, ban_startTime, ban_endTime, record_message
            FROM tbl_playerdata AS pd
            INNER JOIN adkats_bans AS bans ON pd.PlayerId = bans.player_id
            INNER JOIN adkats_records_main AS records ON records.record_id = bans.latest_record_id
            WHERE pd.EAGUID = ? AND ban_status != 'Disabled';"
        , guid.as_ref()).fetch_optional(&self.db).await?;

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

#[cfg(test)]
mod test {
    use anyhow::Context;

    use super::BfoxDb;

    fn get_db_coninfo() -> anyhow::Result<String> {
        dotenv::dotenv()?;
        let uri = std::env::var("DATABASE_URL")
            .context("Need to specify AdKats db URI via env var, for example DATABASE_URL=\"mysql://username:password@host/database\"")?;
        Ok(uri)
    }

    #[ignore]
    #[tokio::test]
    async fn test() -> anyhow::Result<()> {
        let uri = get_db_coninfo()?;
        let db = BfoxDb::new(uri).await?;
        let ban = db.get_ban("EA_insert your GUID here").await?;
        println!("{ban:#?}");
        panic!()
    }
}
