//! Manages map lists based on player population

use std::{sync::Arc, time::Duration};

use battlefield_rcon::{
    bf4::{defs::Preset, Bf4Client, GameMode, Map, Visibility},
    rcon::{RconError, RconResult},
};
use futures::{StreamExt, lock::Mutex};
use tokio::time::{sleep, Instant};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MapChoice {
    pub map: Map,
    pub mode: GameMode,
    pub vehicles: bool,
}

pub struct MapPool {
    maps: Vec<MapChoice>,
}

// impl MapPool {
//     pub fn additions(old: &MapPool, new: &MapPool) {
//     }
//     pub fn removals(old: &MapPool, new: &MapPool) {
//     }
// }

pub enum PopState {
    Seeding,
    LowPop,
    HighPop,
}

pub fn pop_maplist(state: PopState) -> &'static [MapChoice] {
    match state {
        PopState::Seeding => {
            static LIST: [MapChoice; 3] = [
                MapChoice {
                    map: Map::Metro,
                    mode: GameMode::Rush,
                    vehicles: false,
                },
                MapChoice {
                    map: Map::Locker,
                    mode: GameMode::Rush,
                    vehicles: false,
                },
                MapChoice {
                    map: Map::PearlMarket,
                    mode: GameMode::Rush,
                    vehicles: false,
                },
            ];
            &LIST
        }
        PopState::LowPop => {
            static LIST: [MapChoice; 4] = [
                MapChoice {
                    map: Map::Metro,
                    mode: GameMode::Rush,
                    vehicles: false,
                },
                MapChoice {
                    map: Map::Locker,
                    mode: GameMode::Rush,
                    vehicles: false,
                },
                MapChoice {
                    map: Map::PearlMarket,
                    mode: GameMode::Rush,
                    vehicles: false,
                },
                MapChoice {
                    map: Map::Oman,
                    mode: GameMode::Rush,
                    vehicles: false,
                },
            ];
            &LIST
        }
        PopState::HighPop => {
            static LIST: [MapChoice; 4] = [
                MapChoice {
                    map: Map::Metro,
                    mode: GameMode::Rush,
                    vehicles: true,
                },
                MapChoice {
                    map: Map::Locker,
                    mode: GameMode::Rush,
                    vehicles: true,
                },
                MapChoice {
                    map: Map::PearlMarket,
                    mode: GameMode::Rush,
                    vehicles: true,
                },
                MapChoice {
                    map: Map::Oman,
                    mode: GameMode::Rush,
                    vehicles: true,
                },
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
pub struct MapManager {
    maplist: Mutex<Vec<MapChoice>>,
}

impl MapManager {
    pub fn new() -> Self {
        Self {
            maplist: Mutex::new(Vec::new()),
        }
    }

    pub async fn set_maplist(&self, bf4: &Arc<Bf4Client>, popstate: PopState) -> RconResult<()> {
        bf4.maplist_clear().await.unwrap();
        let list = pop_maplist(popstate);

        for (i, mapmode) in list.iter().enumerate() {
            bf4.maplist_add(&mapmode.map, &mapmode.mode, 1, i as i32)
                .await
                .unwrap();
        }

        Ok(())
    }

    pub async fn switch_to(
        &self,
        bf4: &Arc<Bf4Client>,
        map: Map,
        mode: GameMode,
        vehicles: bool,
    ) -> RconResult<()> {
        bf4.maplist_clear().await.unwrap();
        bf4.maplist_add(&map, &mode, 1, 0).await.unwrap();
        bf4.maplist_set_next_map(0).await.unwrap();

        let _ = bf4.set_preset(Preset::Custom).await;
        bf4.set_vehicles_spawn_allowed(vehicles).await.unwrap();

        sleep(Duration::from_secs(1)).await;
        bf4.maplist_run_next_round().await.unwrap();
        sleep(Duration::from_secs(10)).await;

        bf4.set_vehicles_spawn_allowed(true).await.unwrap();
        let _ = bf4.set_preset(Preset::Hardcore).await;

        Ok(())
    }

    pub async fn run(self: Arc<Self>, bf4: Arc<Bf4Client>) -> RconResult<()> {
        bf4.maplist_clear().await.expect("Couldn't clear maplist");

        let n = bf4.list_players(Visibility::All).await.unwrap().len();
        let popstate = count_to_popstate(n);
        self.set_maplist(&bf4, popstate).await.unwrap();

        let mut events = bf4.event_stream().await?;
        while let Some(event) = events.next().await {
            match event {
                _ => {}
            }
        }
        Ok(())
    }
}
