//! A new, better, attempt at battlefox_database with SQLx. Because fuck Diesel.
//!
//! And yet I'm too lazy to get rid of diesel, so that's why I'm just making a `better` module
//! instead of rewriting the whole thing.

// pub struct Config { }

use sea_orm::{DatabaseConnection, Database, EntityTrait, QueryFilter, ColumnTrait, QuerySelect, DbErr};
use thiserror::Error;

use crate::entities::adkats_bans::Entity as AdkatsBans;
use crate::entities::tbl_playerdata::Entity as PlayerData;
use crate::entities::*;

/// Database Connection Pool. Cheap to clone, inner reference counting.
#[derive(Clone)]
pub struct BfoxDb {
    db: DatabaseConnection,
}

#[derive(Error, Debug)]
pub enum BfoxDbError {
    #[error("Uh oh")]
    UhOh,

    #[error("{0}")]
    DbErr(#[from] DbErr),
}

impl BfoxDb {
    /// Connect to database.
    ///
    /// `db_uri` is something like `protocol://username:password@host/database`.
    ///
    /// https://www.sea-ql.org/SeaORM/docs/install-and-config/connection
    pub async fn new(db_uri: impl AsRef<str>) -> Result<Self, anyhow::Error> {
        let db: DatabaseConnection = Database::connect(db_uri.as_ref()).await?;

        Ok(Self {
            db
        })
    }

    /// Check whether the given player GUID is banned, and if yes, return the ban info.
    ///
    /// - `None` means there is no ban record for the player, i.e. not banned.
    /// - `Some((playerdata, ban))` means the player is probably banned, but please still
    ///    check `ban_status`, `end_time`, just to be sure.
    pub async fn get_ban(&self, guid: impl AsRef<str>) -> Result<Option<(tbl_playerdata::Model, adkats_bans::Model)>, BfoxDbError> {
        let ban = PlayerData::find()
            .filter(tbl_playerdata::Column::Eaguid.eq(guid.as_ref()))
            .inner_join(AdkatsBans)
            .select_also(AdkatsBans)
            .one(&self.db).await?;

        let ban = match ban {
            None => None,
            Some((_, None)) => unreachable!("Impossible for inner_join to return only one of two entities."),
            Some((pd, Some(b))) => Some((pd, b)),
        };

        Ok(ban)
    }
}


#[cfg(test)]
mod test {
    use anyhow::Context;

    use super::BfoxDb;

    fn get_db_coninfo() -> anyhow::Result<String> {
        dotenv::dotenv()?;
        let uri = std::env::var("BFOX_ADKATS_URI")
            .context("Need to specify AdKats db URI via env var, for example BFOX_ADKATS_URI=\"mysql://username:password@host/database\"")?;
        Ok(uri)
    }

    // #[ignore]
    #[tokio::test]
    async fn test() -> anyhow::Result<()> {
        let uri = get_db_coninfo()?;
        let db = BfoxDb::new(uri).await?;

        let ban = db.get_ban("EA_23497BD31CD2C20EED45BF21542EA2AD").await?;

        println!("{ban:#?}");

        panic!();
        Ok(())
    }
}