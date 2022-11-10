//! DB connector library with handy bfox-specific data models and queries.
//!
//! This is not a *backend*, it is merely a library which you can use in your app to connect
//! to a MySQL or MariaDB SQL server with a fitting schema.
//!
//! We expose the `BfoxContext` struct, which takes a concrete database connection.
//! This DB connection can be something you provide yourself, for example if you have an existing
//! connection pool, or it can be a new database connection created with `establish_connection`.
//!
//! Different queries for different topics (e.g. banning players, teamkilling, etc..)
//! are implemented in on-topic rust modules, so that this file doens't get too huge.

use std::env;

use sqlx::MySqlPool;
use sqlx::types::time::OffsetDateTime;

pub mod adkats;

pub type DateTime = OffsetDateTime;

pub struct BfoxContext {
    pool: MySqlPool,
}

impl BfoxContext {
    pub fn new_from_pool(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn connect(url: impl AsRef<str>) -> Result<Self, sqlx::Error> {
        let pool = MySqlPool::connect(url.as_ref()).await?; // TODO: unwrap
        Ok(Self { pool })
    }

    /// Create a new `BfoxContext` using a connection string form the `DATABASE_URL` environment
    /// variable, with something like `mysql://username:password@host/database`.
    /// You may need to initialize `dotenv` yourself if you haven't done so yet.
    ///
    /// A connection will only be made when necessary.
    pub fn new_env() -> Self {
        let url = env::var("DATABASE_URL").unwrap(); // TODO: unwrap
        // lazy: will only connect when needed.
        let pool = MySqlPool::connect_lazy(&url).unwrap(); // TODO: unwrap
        Self {
            pool
        }
    }
}

#[cfg(test)]
mod test {
    use anyhow::Context;

    use super::BfoxContext;

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
        let db = BfoxContext::connect(uri).await?;
        let ban = db.get_ban("EA_insert your GUID here").await?;
        println!("{ban:#?}");
        panic!()
    }
}