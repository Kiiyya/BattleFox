//! Manages map lists based on player population

use itertools::Itertools;
use lerp::Lerp;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, convert::TryFrom, sync::Arc, time::Duration};

use battlefield_rcon::{
    bf4::{defs::Preset, Bf4Client, Event, ListPlayersError, Map, MapListError, Visibility},
    rcon::RconResult,
};
use futures::{future::BoxFuture, StreamExt};
use tokio::{sync::Mutex, time::sleep};

use self::{
    config::HasZeroPopState,
    pool::{MapInPool, MapPool, NRounds, Vehicles},
};
use crate::guard::Guard;

pub mod config;
pub mod pool;

/// One "population level", for example for seeding, or for a full server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopState<E: Eq + Clone> {
    pub name: String,
    pub pool: MapPool<E>,
    /// At `min_players` or more players, activate this pool. Unless a pool with even higher `min_players` exists.
    pub min_players: usize,
}

impl<E: Eq + Clone> PopState<E> {
    /// - `Greater`: min players is now higher. Means we changed to a higher pop level.
    /// - etc.
    pub fn change_direction(before: &PopState<E>, after: &PopState<E>) -> Ordering {
        after.min_players.cmp(&before.min_players)
    }
}

/// Find the correct popstate, given a certain population.
pub fn determine_popstate<E: Eq + Clone>(states: &[PopState<E>], pop: usize) -> &PopState<E> {
    if let Some(state) = states
        .iter()
        .filter(|&p| p.min_players <= pop) // if a pop state starts with more players anyway, might as well ignore it.
        .sorted_by(|&a, &b| Ord::cmp(&b.min_players, &a.min_players)) // note swapped a, b in cmp(): We want descending order.
        .next()
    {
        state
    } else {
        panic!("No fitting pop state defined! This is impossible.")
    }
}

/////////////////////////////////////////////
////////// Actual MapManager stuff //////////
/////////////////////////////////////////////

pub struct MapManager {
    inner: Mutex<Inner>,

    pop_states: Vec<PopState<Vehicles>>,
    vehicle_threshold: usize,

    /// Don't want to be overly sensitive to join/leave changes. Amortize it a bit before deciding
    /// to change the pop state.
    /// Unit: Players. For example, 3 players.
    leniency: usize,
}

/// The stuff behind the mutex
struct Inner {
    /// contains the map pool too.
    pop_state: PopState<Vehicles>,

    /// Current amount of players on the server.
    pop: Option<usize>,
    /// Used to steer caching behaviour of `pop`.
    joins_leaves_since_pop: usize,

    #[allow(clippy::type_complexity)]
    pool_change_callbacks: Vec<
        Arc<
            dyn Fn(Arc<Bf4Client>, PopState<Vehicles>) -> BoxFuture<'static, CallbackResult>
                + Send
                + Sync,
        >,
    >,
}

pub enum CallbackResult {
    KeepGoing,
    RemoveMe,
}

impl MapManager {
    pub fn new(
        pop_states: Guard<Vec<PopState<Vehicles>>, HasZeroPopState>,
        vehicle_threshold: usize,
        leniency: usize,
    ) -> Self {
        Self {
            inner: Mutex::new(Inner {
                // pool: MapPool::new(),
                pop_state: pop_states
                    .iter()
                    .find(|state| state.min_players == 0)
                    .unwrap()
                    .clone(), // unwrap: safe because of guard.
                pop: None,
                joins_leaves_since_pop: 0,
                pool_change_callbacks: Vec::new(),
            }),
            pop_states: pop_states.get(),
            vehicle_threshold,
            leniency,
        }
    }

    /// Registers a function to be called when the map pool selection changes.
    ///
    /// Used e.g. in mapvote.
    pub async fn register_pool_change_callback<F>(&self, f: F)
    where
        F: Fn(Arc<Bf4Client>, PopState<Vehicles>) -> BoxFuture<'static, CallbackResult>
            + Send
            + Sync
            + 'static,
    {
        let b = Arc::new(f);
        let mut lock = self.inner.lock().await;
        lock.pool_change_callbacks.push(b);
    }

    /// Checks whether the map is in the current pool.
    pub async fn is_in_current_pool(&self, map: Map) -> bool {
        self.inner.lock().await.pop_state.pool.contains_map(map)
    }

    /// Switches to the new map and game mode, but optionally disables vehicle spawns with
    /// the RCON trick. (Server will be custom for about 10 seconds).
    pub async fn switch_to(
        &self,
        bf4: &Arc<Bf4Client>,
        mip: &MapInPool<()>,
    ) -> Result<(), MapListError> {
        let pop = self.get_pop_count(bf4).await?;
        let vehicles = pop >= self.vehicle_threshold;

        let pop_state = {
            let lock = self.inner.lock().await;
            lock.pop_state.clone()
        };

        let tickets = match pop {
            x if x <= 8 => 75_f64,
            x if x <= 16 => 75_f64.lerp_bounded(120_f64, (x as f64 - 8_f64) / 8_f64),
            x if x <= 32 => 120_f64.lerp_bounded(250_f64, (x as f64 - 16_f64) / 16_f64),
            x if x <= 64 => 250_f64.lerp_bounded(400_f64, (x as f64 - 32_f64) / 16_f64),
            _ => 400_f64,
        };
        // let tickets = ()
        let tickets = tickets as usize;

        if let Some(index) = pop_state.pool.get_rcon_index(mip.map, &mip.mode, |_| true) {
            // sweet, index is valid. Go for it.
            switch_map_to(bf4, index, vehicles, tickets).await?;
            Ok(())
        } else {
            println!("Failed to find RCON index of {} {:?}. This is possible, but should not happen tooo often.", mip.map.Pretty(), mip.mode);
            // just add the map temporarily and switch anyway.
            bf4.maplist_add(&mip.map, &mip.mode, 1, 0).await?;
            switch_map_to(bf4, 0, vehicles, tickets).await?;
            bf4.maplist_remove(0).await?;
            Ok(())
        }
    }

    /// Gets the cached amount of players currently on the server, or fetches it by listing all
    /// players via RCON.
    pub async fn get_pop_count(&self, bf4: &Arc<Bf4Client>) -> RconResult<usize> {
        let lock = self.inner.lock().await;
        if let Some(pop) = lock.pop {
            Ok(pop)
        } else {
            drop(lock); // don't keep it locked while we query rcon.
            let playerlist = match bf4.list_players(Visibility::All).await {
                Ok(list) => list,
                Err(ListPlayersError::Rcon(rconerr)) => return Err(rconerr),
            };
            let new_pop = playerlist.len();
            let mut lock = self.inner.lock().await;
            lock.pop = Some(new_pop);
            lock.joins_leaves_since_pop = 0;
            Ok(new_pop)
        }
    }

    /// Call this when someone joins/leaves and it'll auto update
    async fn pop_change(&self, change: isize, bf4: &Arc<Bf4Client>) -> Result<(), MapListError> {
        let mut lock = self.inner.lock().await;
        // every 5 joins/leaves, we yeet the pop count, to prevent it desyncing.
        if lock.joins_leaves_since_pop > 5 {
            // fuck it, next time someone calls `get_pop_count`, it'll update.
            lock.pop = None;
            lock.joins_leaves_since_pop = 0;
        } else {
            lock.joins_leaves_since_pop += 1;
            if let Some(pop) = &mut lock.pop {
                // this is fine due to 2s-complement
                // but it can still happen that rcon somehow ate join/leave packets, so juuuuust
                // in caaaase that happened, we need to prevent negative pop counts.
                *pop = pop.wrapping_add(change as usize);

                if *pop > 9999 {
                    lock.pop = Some(0);
                }
            }
        }
        drop(lock);

        // now, check if we need to change the pop_state.
        let pop = self.get_pop_count(bf4).await?; // get true player count.
        let next_state = determine_popstate(&self.pop_states, pop);

        let lock = self.inner.lock().await;
        let current_state = lock.pop_state.clone();
        if next_state.name == current_state.name {
            // we're exactly where we should be. nice, nothing to do.
            drop(lock);
            Ok(())
        } else {
            let pop = isize::try_from(pop).unwrap();
            let diff_current = pop - isize::try_from(lock.pop_state.min_players).unwrap(); // +: pop higher than min players, -: pop fell below min_players.
            let _diff_next = pop - isize::try_from(next_state.min_players).unwrap();
            drop(lock);

            let leniency = isize::try_from(self.leniency).unwrap();
            if diff_current < -leniency {
                // pop fell below the current state's min_players. Switch to proper pop state.
                // This also means we're going "down"
                self.change_pop_state(next_state.clone(), bf4).await?;
                Ok(())
            } else if diff_current > leniency {
                // we have reached into the next pop state far enough (leniency), that we can decide to switch to it.
                self.change_pop_state(next_state.clone(), bf4).await?;
                Ok(())
            } else {
                // nothing to do, leniency not yet exceeded.
                Ok(())
            }
        }
    }

    /// Gets the current population state
    /// The amortized one. This is **not** necessarily equal to getting current population, and then
    /// `determine_popstate`!
    pub async fn pop_state(&self) -> PopState<Vehicles> {
        let lock = self.inner.lock().await;
        lock.pop_state.clone()
    }

    /// Change pop state (locking inner), swap out RCON maplist, and call all handlers.
    pub async fn change_pop_state(
        &self,
        newpop: PopState<Vehicles>,
        bf4: &Arc<Bf4Client>,
    ) -> Result<(), MapListError> {
        // fill maps, set one round on each map (we ignore rounds, but RCON needs something).
        // In turn, RCON isn't aware of vehicles yes/no...
        fill_rcon_maplist(bf4, &newpop.pool.map_to_nrounds(|_| 2)).await?;

        let mut lock = self.inner.lock().await;
        lock.pop_state = newpop.clone();
        let handlers = lock.pool_change_callbacks.clone();
        drop(lock); // since handlers may need to make very long rcon calls, drop lock early.

        let mut yeet_handlers = Vec::new();
        // notify observers
        for handler in handlers {
            match handler(bf4.clone(), newpop.clone()).await {
                CallbackResult::KeepGoing => {}
                CallbackResult::RemoveMe => {
                    dbg!(yeet_handlers.push(handler));
                }
            }
        }

        // TODO: actually remove them
        // let mut lock = self.inner.lock().await;
        // for handler in yeet_handlers {
        //     lock.pool_change_callbacks.remove(index)
        // }

        Ok(())
    }

    pub async fn run(self: Arc<Self>, bf4: Arc<Bf4Client>) -> RconResult<()> {
        // on start, get current player amounts (pop), then switch to that popstate initially.
        // In the constructor, popstate gets set to the base (0) case, but when we launch BattleFox,
        // it may not be on an empty server.
        let pop = self.get_pop_count(&bf4).await?;
        let state = determine_popstate(&self.pop_states, pop).clone();
        self.change_pop_state(state, &bf4)
            .await
            .map_err(|mle| match mle {
                MapListError::Rcon(rcon) => rcon,
                MapListError::MapListFull => panic!("Map list full, huh!"),
                MapListError::InvalidGameMode => panic!("Invalid game mode, huh!"),
                MapListError::InvalidMapIndex => panic!("Invalid map index, huh!"),
                MapListError::InvalidRoundsPerMap => panic!("Invalid rounds per map, huh!"),
            })?;

        let mut events = bf4.event_stream().await?;
        while let Some(event) = events.next().await {
            match event {
                // Join also catches the seeder bots joining.
                // Authenticated doesn't (hopefully).
                Ok(Event::Authenticated { player: _ }) => {
                    self.pop_change(1, &bf4).await.map_err(|mle| match mle {
                        MapListError::Rcon(rcon) => rcon,
                        MapListError::MapListFull => panic!("Map list full, huh!"),
                        MapListError::InvalidGameMode => panic!("Invalid game mode, huh!"),
                        MapListError::InvalidMapIndex => panic!("Invalid map index, huh!"),
                        MapListError::InvalidRoundsPerMap => panic!("Invalid rounds per map, huh!"),
                    })?
                }
                Ok(Event::Leave {
                    player: _,
                    final_scores: _,
                }) => self.pop_change(-1, &bf4).await.map_err(|mle| match mle {
                    MapListError::Rcon(rcon) => rcon,
                    MapListError::MapListFull => panic!("Map list full, huh!"),
                    MapListError::InvalidGameMode => panic!("Invalid game mode, huh!"),
                    MapListError::InvalidMapIndex => panic!("Invalid map index, huh!"),
                    MapListError::InvalidRoundsPerMap => panic!("Invalid rounds per map, huh!"),
                })?,
                _ => {}
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for MapManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(lock) = self.inner.try_lock() {
            write!(f, "MapManager {{ pool: {:?}, pop: {:?}, joins_leaves_since_pop: {:?}, pool_change_callbacks.len(): {} }}",
                lock.pop_state.pool, lock.pop, lock.joins_leaves_since_pop, lock.pool_change_callbacks.len())
        } else {
            f.write_str("MapManager { locked inner }")
        }
    }
}

/// Clears and then fills the rcon maplist (as seen on battlelog and procon) with the specified map pool.
pub async fn fill_rcon_maplist(
    bf4: &Arc<Bf4Client>,
    pool: &MapPool<NRounds>,
) -> Result<(), MapListError> {
    bf4.maplist_clear().await?;
    for (offset, mip) in pool.pool.iter().enumerate() {
        bf4.maplist_add(&mip.map, &mip.mode, mip.extra.0 as i32, offset as i32)
            .await?;
    }

    Ok(())
}

pub async fn switch_map_to(
    bf4: &Arc<Bf4Client>,
    index: usize,
    vehicles: bool,
    tickets: usize,
) -> Result<(), MapListError> {
    bf4.maplist_set_next_map(index).await?;
    println!("[mapman switch_map_to()] index: {}, vehicles: {}, tickets: {}", index, vehicles, tickets);

    let _ = bf4.set_preset(Preset::Custom).await;
    let _ = bf4.set_vehicles_spawn_allowed(vehicles).await;

    // Force the vehicle spawn delay to the default value, 100.
    // We do this as a safeguard against previously seen quirks,
    // where the value would be automatically set to 400.
    let _ = bf4.set_vehicle_spawn_delay(100).await;

    let _  = dbg!(bf4.set_tickets(tickets).await);
    sleep(Duration::from_secs(1)).await;

    bf4.maplist_run_next_round().await?;

    sleep(Duration::from_secs(10)).await;
    let _  = dbg!(bf4.set_tickets(std::cmp::max(100, tickets)).await);
    let _ = bf4.set_vehicle_spawn_delay(100).await;
    let _ = bf4.set_vehicles_spawn_allowed(true).await;
    let _ = bf4.set_preset(Preset::Hardcore).await;

    Ok(())
}

/// Fetch the maplist in RCON and return it.
pub async fn read_rcon_pool(bf4: &Arc<Bf4Client>) -> Result<MapPool<NRounds>, MapListError> {
    let list = bf4.maplist_list().await?;
    let mut pool = MapPool::new();
    for mle in list {
        pool.pool.push(MapInPool {
            map: mle.map,
            mode: mle.game_mode,
            extra: NRounds(mle.n_rounds),
        });
    }

    Ok(pool)
}
