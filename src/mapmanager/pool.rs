use std::fmt::Display;

use battlefield_rcon::bf4::{GameMode, Map};
use serde::{Deserialize, Serialize};

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

impl<E: Eq + Clone> Display for MapInPool<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.map.Pretty())
    }
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

    /// Attempts to get the index of the map in the map pool.
    pub fn get_rcon_index(
        &self,
        map: Map,
        mode: &GameMode,
        extra_matcher: impl Fn(&MapInPool<E>) -> bool,
    ) -> Option<usize> {
        self.pool
            .iter()
            .enumerate()
            .filter(|(_, mip)| mip.map == map && &mip.mode == mode && extra_matcher(&mip))
            .map(|(i, _)| i)
            .next()
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
