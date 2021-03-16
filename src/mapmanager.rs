//! Manages map lists based on player population

use std::{sync::{Arc, Mutex}, time::Duration};

use battlefield_rcon::{bf4::{Bf4Client, Event, GameMode, ListPlayersError, Map, Visibility, defs::Preset, error::Bf4Error}, rcon::{RconError, RconResult}};
use futures::StreamExt;
use tokio::time::{sleep, Instant};

/// A map in a map pool.
/// Simple Triple of
/// - map
/// - mode
/// - vehicles yes/no.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MapInPool {
    pub map: Map,
    pub mode: GameMode,
    pub vehicles: bool,
}

/// Helper struct to make diffing map pools easier.
#[derive(Debug)]
pub struct MapPool {
    pub pool: Vec<MapInPool>,
}

impl MapPool {
    pub fn new() -> Self {
        Self {
            pool: Vec::new(),
        }
    }

    /// Checks whether map exists in this map pool.
    pub fn is_in(&self, map: Map) -> bool {
        self.pool.iter().any(|mip| mip.map == map)
    }

    /// Returns a new pool, with only the maps which are new in `new`.
    pub fn additions(old: &MapPool, new: &MapPool) -> MapPool {
        MapPool {
            pool: new.pool.iter().filter(|new_mip| !old.is_in(new_mip.map)).cloned().collect(),
        }
    }

    /// Returns a new pool, with only the removed maps in `new`.
    pub fn removals(old: &MapPool, new: &MapPool) -> MapPool {
        MapPool {
            pool: old.pool.iter().filter(|old_mip| !new.is_in(old_mip.map)).cloned().collect(),
        }
    }
}

#[derive(Debug)]
pub enum PopState {
    Seeding,
    LowPop,
    HighPop,
}

pub fn pop_maplist(state: PopState) -> &'static [MapInPool] {
    match state {
        PopState::Seeding => {
            static LIST: [MapInPool; 3] = [
                MapInPool {
                    map: Map::Metro,
                    mode: GameMode::Rush,
                    vehicles: false,
                },
                MapInPool {
                    map: Map::Locker,
                    mode: GameMode::Rush,
                    vehicles: false,
                },
                MapInPool {
                    map: Map::PearlMarket,
                    mode: GameMode::Rush,
                    vehicles: false,
                },
            ];
            &LIST
        }
        PopState::LowPop => {
            static LIST: [MapInPool; 4] = [
                MapInPool {
                    map: Map::Metro,
                    mode: GameMode::Rush,
                    vehicles: false,
                },
                MapInPool {
                    map: Map::Locker,
                    mode: GameMode::Rush,
                    vehicles: false,
                },
                MapInPool {
                    map: Map::PearlMarket,
                    mode: GameMode::Rush,
                    vehicles: false,
                },
                MapInPool {
                    map: Map::Oman,
                    mode: GameMode::Rush,
                    vehicles: false,
                },
            ];
            &LIST
        }
        PopState::HighPop => {
            static LIST: [MapInPool; 4] = [
                MapInPool {
                    map: Map::Metro,
                    mode: GameMode::Rush,
                    vehicles: true,
                },
                MapInPool {
                    map: Map::Locker,
                    mode: GameMode::Rush,
                    vehicles: true,
                },
                MapInPool {
                    map: Map::PearlMarket,
                    mode: GameMode::Rush,
                    vehicles: true,
                },
                MapInPool {
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

struct Inner {
    pool: MapPool,
    pop: Option<usize>,
    joins_leaves_since_pop: usize,

    pool_change_callbacks: Vec<Box<dyn Fn(MapPool) + Send>>,
}

impl std::fmt::Debug for Inner {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[derive(Debug)]
pub struct MapManager {
    inner: Mutex<Inner>,
}

impl MapManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                pool: MapPool::new(),
                pop: Some(0),
                joins_leaves_since_pop: 0,
                pool_change_callbacks: Vec::new(),
            })
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

    /// Registers a function to be called when the map pool selection changes.
    /// 
    /// Used e.g. in mapvote.
    pub fn register_pool_change_callback<F: Fn(MapPool) + Send + 'static>(&self, f: F) {
        let mut lock = self.inner.lock().unwrap();
        lock.pool_change_callbacks.push(Box::new(f));
    }

    /// Checks whether the map is in the current pool.
    pub fn is_in_current_pool(&self, map: Map) -> bool {
        self.inner.lock().unwrap().pool.is_in(map)
    }

    /// Switches to the new map and game mode, but optionally disables vehicle spawns with
    /// the RCON trick. (Server will be custom for about 10 seconds).
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

    /// Gets the cached amount of players currently on the server, or fetches it by listing all
    /// players via RCON.
    pub async fn get_pop_count(&self, bf4: &Arc<Bf4Client>) -> RconResult<usize> {
        let lock = self.inner.lock().unwrap();
        if let Some(pop) = lock.pop {
            Ok(pop)
        } else {
            drop(lock); // don't keep it locked while we query rcon.
            let playerlist = match bf4.list_players(Visibility::All).await {
                Ok(list) => list,
                Err(ListPlayersError::Rcon(rconerr)) => return Err(rconerr),
            };
            let new_pop = dbg!(playerlist.len());
            let mut lock = self.inner.lock().unwrap();
            lock.pop = Some(new_pop);
            lock.joins_leaves_since_pop = 0;
            Ok(new_pop)
        }
    }

    async fn pop_change(&self, change: isize) {
        let mut inner = self.inner.lock().unwrap();
        // every 20 joins/leaves, we yeet the pop count, to prevent it desyncing.
        if inner.joins_leaves_since_pop > 20 {
            // fuck it, next time someone calls `get_pop_count`, it'll update.
            inner.pop = None;
            inner.joins_leaves_since_pop = 0;
        } else {
            inner.joins_leaves_since_pop += 1;
            if let Some(pop) = &mut inner.pop {
                *pop += change as usize; // this is fine due to 2s-complement
                // but it can still happen that rcon somehow ate join/leave packets, so juuuuust
                // in caaaase that happened, we need to prevent negative pop counts.
                if *pop > 9999 {
                    inner.pop = None;
                    // fuck it, next time someone calls `get_pop_count`, it'll update.
                }
            }
        }
    }

    pub async fn run(self: Arc<Self>, bf4: Arc<Bf4Client>) -> RconResult<()> {
        bf4.maplist_clear().await.expect("Couldn't clear maplist");

        let n = bf4.list_players(Visibility::All).await.unwrap().len();
        let popstate = count_to_popstate(n);
        self.set_maplist(&bf4, popstate).await.unwrap();

        let mut events = bf4.event_stream().await?;
        while let Some(event) = events.next().await {
            match event {
                Ok(Event::Join { player: _ }) => self.pop_change(1).await,
                Ok(Event::Leave { player: _ }) => self.pop_change(-1).await,
                Err(Bf4Error::Rcon(RconError::ConnectionClosed)) => break,
                _ => todo!()
            }
        }
        Ok(())
    }
}
