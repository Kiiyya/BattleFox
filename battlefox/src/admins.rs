//! Provides info on which player is an admin.

use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use battlefield_rcon::bf4::Bf4Client;
use serde::Deserialize;

use crate::Plugin;


#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    admins: BTreeSet<String>,
}

pub struct Admins {
    config: Config,
}

impl Admins {
    pub fn new(cfg: Config) -> Self {
        todo!()
    }
}

#[async_trait]
impl Plugin for Admins {
    fn name() -> &'static str { "admins" }
    async fn run(self: Arc<Self>, bf4: Arc<Bf4Client>) {
        todo!()
    }
}
