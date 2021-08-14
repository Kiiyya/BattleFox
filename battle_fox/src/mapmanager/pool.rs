use std::{collections::HashSet, fmt::Display, hash::Hash, slice::Iter};

use battlefield_rcon::bf4::{GameMode, Map};
use rand::{prelude::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};

/// A map in a map pool.
/// Simple Triple of
/// - map
/// - game mode (Rush, Conquest, ...)
/// - vehicles (None -> Adaptive based on vehicle_threshold, False -> No vehicles, True -> Vehicles)
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, AsRef)]
pub struct MapInPool {
    pub map: Map,
    pub mode: GameMode,
    pub vehicles: Option<bool>
}

impl Display for MapInPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.map.Pretty())
    }
}

impl std::fmt::Debug for MapInPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.map.short(), self.mode)
    }
}

/// Helper struct to make diffing map pools easier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MapPool {
    pub pool: Vec<MapInPool>,
}

impl Default for MapPool {
    fn default() -> Self {
        Self { pool: Vec::new() }
    }
}

impl From<Vec<MapInPool>> for MapPool {
    fn from(pool: Vec<MapInPool>) -> Self {
        Self {
            pool
        }
    }
}
impl From<Vec<(Map, GameMode, Option<bool>)>> for MapPool {
    fn from(pool: Vec<(Map, GameMode, Option<bool>)>) -> Self {
        Self {
            pool: pool.iter().map(|(map, mode, vehicles)| MapInPool {
                map: *map,
                mode: mode.clone(),
                vehicles: *vehicles
            }).collect()
        }
    }
}

impl MapPool {
    pub fn new() -> Self {
        Self { pool: Vec::new() }
    }

    pub fn iter(&self) -> Iter<MapInPool> {
        self.pool.iter()
    }

    /// Checks whether map exists in this map pool.
    pub fn contains_map(&self, map: Map) -> bool {
        self.pool.iter().any(|mip| mip.map == map)
    }

    /// Checks whether the `(map, mode)` combination exists in the pool, ignoring `extra`.
    pub fn contains_mapmode(&self, map: Map, mode: &GameMode) -> bool {
        self.pool
            .iter()
            .any(|mip| mip.map == map && &mip.mode == mode)
    }

    /// Attempts to get the index of the map in the map pool.
    pub fn get_rcon_index(
        &self,
        map: Map,
        mode: &GameMode,
        extra_matcher: impl Fn(&MapInPool) -> bool,
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
    /// Only considers the `map` and `mode` fields, ignoreing extra.
    pub fn additions(old: &Self, new: &Self) -> Self {
        Self {
            pool: new
                .pool
                .iter()
                .filter(|new_mip| !old.contains_mapmode(new_mip.map, &new_mip.mode))
                .cloned()
                .collect(),
        }
    }

    /// Returns a new pool, with only the removed maps in `new`.
    ///
    /// Only considers the `map` and `mode` fields, ignoreing extra.
    pub fn removals(old: &Self, new: &Self) -> Self {
        Self {
            pool: old
                .pool
                .iter()
                .filter(|old_mip| !new.contains_mapmode(old_mip.map, &old_mip.mode))
                .cloned()
                .collect(),
        }
    }

    /// Only considers the `map` and `mode` fields, ignoreing extra.
    pub fn intersect(&self, other: &Self) -> Self {
        Self {
            pool: self
                .pool
                .iter()
                .filter(|&selfmip| other.contains_mapmode(selfmip.map, &selfmip.mode))
                .cloned()
                .collect(),
        }
    }

    pub fn to_set(&self) -> HashSet<MapInPool>
    {
        self.pool.iter().cloned().collect()
    }

    pub fn to_mapset(&self) -> HashSet<Map> {
        self.pool.iter().map(|mip| mip.map).collect()
    }

    // /// Returns the maps which retain the same (Map, Mode), but whose `extra` changed.
    // ///
    // /// # Returns
    // /// List of tuples of
    // /// - [`MapInPool<E>`] with the *new* extra.
    // /// - The old extra.
    // pub fn changes<'old, 'new>(
    //     old: &'old Self,
    //     new: &'new Self,
    // ) -> Vec<(&'new MapInPool<E>)> {
    //     let mut vec: Vec<(&'new MapInPool<E>, &'old E)> = Vec::new();

    //     for new_mip in &new.pool {
    //         if let Some(old_mip) = old.pool.iter().find(|old_mip| {
    //             old_mip.map == new_mip.map
    //                 && old_mip.mode == new_mip.mode
    //                 && old_mip.extra != new_mip.extra
    //         }) {
    //             vec.push((new_mip, &old_mip.extra))
    //         }
    //     }

    //     vec
    // }

    /// Selects at most `n_max` maps from the pool at random.
    pub fn choose_random(&self, n_max: usize) -> Self {
        let mut rng = thread_rng();
        Self {
            pool: self
                .pool
                .choose_multiple(&mut rng, n_max)
                .cloned()
                .collect(),
        }
    }

    /// Returns a new map pool which contains the same items, except with any with `map` removed.
    pub fn without(&self, map: Map) -> Self {
        Self {
            pool: self
                .pool
                .iter()
                .filter(|&mip| mip.map != map)
                .cloned()
                .collect(),
        }
    }

    /// Returns a new map pool which contains the same items, except with any with `map` removed.
    pub fn without_many(&self, maps: &HashSet<Map>) -> Self {
        Self {
            pool: self
                .pool
                .iter()
                .filter(|&mip| !maps.contains(&mip.map))
                .cloned()
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use battlefield_rcon::bf4::GameMode;

    use super::*;

    #[test]
    fn additions() {
        let p1 = MapPool {
            pool: vec![MapInPool {
                map: Map::Metro,
                mode: GameMode::Rush,
                vehicles: None,
            }],
        };

        let p2 = MapPool {
            pool: vec![
                MapInPool {
                    map: Map::Metro,
                    mode: GameMode::Rush,
                    vehicles: None,
                },
                MapInPool {
                    map: Map::Locker,
                    mode: GameMode::Rush,
                    vehicles: None,
                },
            ],
        };

        let p_addition = MapPool {
            pool: vec![MapInPool {
                map: Map::Locker,
                mode: GameMode::Rush,
                vehicles: None,
            }],
        };

        assert_eq!(p_addition, MapPool::additions(&p1, &p2));
        assert_eq!(MapPool::default(), MapPool::removals(&p1, &p2)); // no removals
    }

    #[test]
    fn removals() {
        let p1 = MapPool {
            pool: vec![
                MapInPool {
                    map: Map::Metro,
                    mode: GameMode::Rush,
                    vehicles: None,
                },
                MapInPool {
                    map: Map::Locker,
                    mode: GameMode::Rush,
                    vehicles: None,
                },
            ],
        };

        let p2 = MapPool {
            pool: vec![MapInPool {
                map: Map::Metro,
                mode: GameMode::Rush,
                vehicles: None,
            }],
        };

        let p_removal = MapPool {
            pool: vec![MapInPool {
                map: Map::Locker,
                mode: GameMode::Rush,
                vehicles: None,
            }],
        };

        assert_eq!(p_removal, MapPool::removals(&p1, &p2));
        assert_eq!(MapPool::default(), MapPool::additions(&p1, &p2)); // no additions
    }
}
