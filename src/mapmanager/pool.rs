use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{io::SeekFrom, sync::Arc, time::Duration};

use battlefield_rcon::{
    bf4::{
        defs::Preset, error::Bf4Error, Bf4Client, Event, GameMode, ListPlayersError, Map,
        MapListError, Visibility,
    },
    rcon::{RconError, RconResult},
};
use futures::{future::BoxFuture, Future, StreamExt};
use tokio::{
    sync::Mutex,
    time::{sleep, Instant},
};

use crate::guard::{Guard, Judgement};

/// A map in a map pool.
/// Simple Triple of
/// - map
/// - game mode (Rush, Conquest, ...)
/// - extra meta stuff (e.g. whether vehicles are enabled yes/no.)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MapInPool<E: Eq + Clone> {
    pub map: Map,
    pub mode: GameMode,
    pub extra: E,
}

/// Whether the MapInPool makes any claim about whether to use vehicles or not.
pub trait VehiclesSpecified {
    fn has_vehicles(&self) -> bool;
}
/// Carries information whether the MIP has vehicles specified or not. And carries whether there
/// should be vehicles or not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vehicles(bool);
impl VehiclesSpecified for MapInPool<Vehicles> {
    fn has_vehicles(&self) -> bool {
        self.extra.0
    }
}

pub trait HasRounds {
    fn has_rounds(&self) -> usize;
}
/// Amount of rounds that a map in a map pool has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NRounds(pub usize);
impl HasRounds for MapInPool<NRounds> {
    fn has_rounds(&self) -> usize {
        self.extra.0
    }
}

/// Helper struct to make diffing map pools easier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MapPool<E: Eq + Clone> {
    pub pool: Vec<MapInPool<E>>,
}

impl<E: Eq + Clone> MapPool<E> {
    pub fn new() -> Self {
        Self { pool: Vec::new() }
    }

    /// Checks whether map exists in this map pool.
    pub fn contains_map(&self, map: Map) -> bool {
        self.pool.iter().any(|mip| mip.map == map)
    }

    /// Returns a new pool, with only the maps which are new in `new`.
    ///
    /// Only considers the `map` field, ignore gamemode, extra, etc...
    pub fn additions(old: &Self, new: &Self) -> Self {
        Self {
            pool: new
                .pool
                .iter()
                .filter(|new_mip| !old.contains_map(new_mip.map))
                .cloned()
                .collect(),
        }
    }

    /// Returns a new pool, with only the removed maps in `new`.
    ///
    /// Only considers the `map` field, ignore gamemode, extra, etc...
    pub fn removals(old: &Self, new: &Self) -> Self {
        Self {
            pool: old
                .pool
                .iter()
                .filter(|old_mip| !new.contains_map(old_mip.map))
                .cloned()
                .collect(),
        }
    }

    /// For example `pool.map_to_nrounds(|_| 1)` to just get one round per map.
    pub fn map_to_nrounds(&self, f: impl Fn(&MapInPool<E>) -> usize) -> MapPool<NRounds> {
        MapPool {
            pool: self
                .pool
                .iter()
                .map(|mip| MapInPool::<NRounds> {
                    map: mip.map,
                    mode: mip.mode.clone(),
                    extra: NRounds(f(mip)),
                })
                .collect(),
        }
    }
}
