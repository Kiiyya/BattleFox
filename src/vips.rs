use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use battlefield_rcon::bf4::{Bf4Client, Player};

struct CacheLine {
    vip: bool,
}

pub struct Vips {
    cache: Mutex<HashMap<Player, CacheLine>>,
}

impl Vips {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    // pub async fn resolve_vip(&self, bf4: &Arc<Bf4Client>, player: &Player) -> bool {
    //     let hit
    // }

    pub async fn set_vip(&self, player: &Player, vip: bool) {}
    pub async fn flush_cache(&self, key: &Player) {}
}

