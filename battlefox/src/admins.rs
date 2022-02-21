//! Provides info on which player is an admin.

use std::collections::BTreeSet;

use async_trait::async_trait;

use crate::{Plugin};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    admins: BTreeSet<String>,
}

#[derive(Debug)]
pub struct Admins {
    config: Config,
}

impl Admins {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn is_admin(&self, player_name: impl AsRef<str>) -> bool {
        self.config.admins.contains(player_name.as_ref())
    }
}

#[async_trait]
impl Plugin for Admins {
    const NAME: &'static str = "admins";

    // async fn event(self: Arc<Self>, _bf4: Arc<Bf4Client>, _ev: Event) -> RconResult<()> {
    //     // do nothing unless overridden.
    //     Ok(())
    // }
}
