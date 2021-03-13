//! Manages map lists based on player population

use std::sync::Arc;

use battlefield_rcon::{bf4::{Bf4Client, GameMode, Map}, rcon::RconResult};
use futures::lock::Mutex;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MapMode {
    pub map: Map,
    pub mode: GameMode,
    pub vehicles: bool,
}

pub enum PopState {
    Seeding,
    LowPop,
    HighPop,
}

pub fn pop_maplist(state: PopState) -> &'static [MapMode] {
    match state {
        PopState::Seeding => {
            static LIST: [MapMode; 3] = [
                MapMode {map: Map::Metro, mode: GameMode::Rush, vehicles: false},
                MapMode {map: Map::Locker, mode: GameMode::Rush, vehicles: false},
                MapMode {map: Map::PearlMarket, mode: GameMode::Rush, vehicles: false},
            ];
            &LIST
        }
        PopState::LowPop => {
            static LIST: [MapMode; 4] = [
                MapMode {map: Map::Metro, mode: GameMode::Rush, vehicles: false},
                MapMode {map: Map::Locker, mode: GameMode::Rush, vehicles: false},
                MapMode {map: Map::PearlMarket, mode: GameMode::Rush, vehicles: false},
                MapMode {map: Map::Oman, mode: GameMode::Rush, vehicles: false},
            ];
            &LIST
        },
        PopState::HighPop => {
            static LIST: [MapMode; 4] = [
                MapMode {map: Map::Metro, mode: GameMode::Rush, vehicles: false},
                MapMode {map: Map::Locker, mode: GameMode::Rush, vehicles: false},
                MapMode {map: Map::PearlMarket, mode: GameMode::Rush, vehicles: false},
                MapMode {map: Map::Oman, mode: GameMode::Rush, vehicles: true},
            ];
            &LIST
        }
    }
}

pub fn count_to_popstate(count: usize) -> PopState {
    match count {
        0..=10 => PopState::Seeding,
        11..=27 => PopState::LowPop,
        _ => PopState::HighPop,
    }
}

#[derive(Debug)]
pub struct MapList {
    maplist: Mutex<Vec<MapMode>>,
}

impl MapList {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            maplist: Mutex::new(Vec::new()),
        }
    }

    pub async fn set_maplist(&self, bf4: &Arc<Bf4Client>, popstate: PopState) -> RconResult<()> {
        bf4.maplist_clear().await;
        

        Ok(())
    }

    pub async fn init(&self, bf4: &Arc<Bf4Client>) -> RconResult<()> {
        bf4.maplist_clear().await.expect("Couldn't clear maplist");

        Ok(())
    }
}

