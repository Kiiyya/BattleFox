use std::{collections::HashSet, fmt::Display, hash::Hash};

use battlefield_rcon::bf4::{GameMode, Map};
use rand::{prelude::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};

/// A map in a map pool.
/// Simple Triple of
/// - map
/// - game mode (Rush, Conquest, ...)
/// - extra meta stuff (e.g. whether vehicles are enabled yes/no.)
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl <E: Eq + Clone + std::fmt::Debug> std::fmt::Debug for MapInPool<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}@{:?}", self.map.short(), self.mode, self.extra)
    }
}

/// Whether the MapInPool makes any claim about whether to use vehicles or not.
pub trait VehiclesSpecified {
    fn has_vehicles(&self) -> bool;
}
/// Carries information whether the MIP has vehicles specified or not. And carries whether there
/// should be vehicles or not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vehicles(pub bool);
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

impl<E: Eq + Clone> Default for MapPool<E> {
    fn default() -> Self {
        Self { pool: Vec::new() }
    }
}

impl <E: Eq + Clone> From<Vec<MapInPool<E>>> for MapPool<E> {
    fn from(pool: Vec<MapInPool<E>>) -> Self {
        Self {
            pool
        }
    }
}
impl From<Vec<(Map, GameMode)>> for MapPool<()> {
    fn from(pool: Vec<(Map, GameMode)>) -> Self {
        Self {
            pool: pool.iter().map(|(map, mode)| MapInPool {
                map: *map,
                mode: mode.clone(),
                extra: (),
            }).collect()
        }
    }
}

impl<E: Eq + Clone> MapPool<E> {
    pub fn new() -> Self {
        Self { pool: Vec::new() }
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

    pub fn to_set(&self) -> HashSet<MapInPool<E>>
    where
        E: Hash,
    {
        self.pool.iter().cloned().collect()
    }

    pub fn to_mapset(&self) -> HashSet<Map> {
        self.pool.iter().map(|mip| mip.map).collect()
    }

    /// Returns the maps which retain the same (Map, Mode), but whose `extra` changed.
    ///
    /// # Returns
    /// List of tuples of
    /// - [`MapInPool<E>`] with the *new* extra.
    /// - The old extra.
    pub fn changes<'old, 'new>(
        old: &'old Self,
        new: &'new Self,
    ) -> Vec<(&'new MapInPool<E>, &'old E)> {
        let mut vec: Vec<(&'new MapInPool<E>, &'old E)> = Vec::new();

        for new_mip in &new.pool {
            if let Some(old_mip) = old.pool.iter().find(|old_mip| {
                old_mip.map == new_mip.map
                    && old_mip.mode == new_mip.mode
                    && old_mip.extra != new_mip.extra
            }) {
                vec.push((new_mip, &old_mip.extra))
            }
        }

        vec
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

    pub fn map_extra<E2: Eq + Clone>(&self, f: impl Fn(&MapInPool<E>) -> E2) -> MapPool<E2> {
        MapPool {
            pool: self
                .pool
                .iter()
                .map(|mip| MapInPool::<E2> {
                    map: mip.map,
                    mode: mip.mode.clone(),
                    extra: f(mip),
                })
                .collect(),
        }
    }

    pub fn extra_remove(&self) -> MapPool<()> {
        self.map_extra(|_| ())
    }

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
        let p1 = MapPool::<()> {
            pool: vec![MapInPool {
                map: Map::Metro,
                mode: GameMode::Rush,
                extra: (),
            }],
        };

        let p2 = MapPool::<()> {
            pool: vec![
                MapInPool {
                    map: Map::Metro,
                    mode: GameMode::Rush,
                    extra: (),
                },
                MapInPool {
                    map: Map::Locker,
                    mode: GameMode::Rush,
                    extra: (),
                },
            ],
        };

        let p_addition = MapPool::<()> {
            pool: vec![MapInPool {
                map: Map::Locker,
                mode: GameMode::Rush,
                extra: (),
            }],
        };

        assert_eq!(p_addition, MapPool::additions(&p1, &p2));
        assert_eq!(MapPool::default(), MapPool::removals(&p1, &p2)); // no removals
    }

    #[test]
    fn removals() {
        let p1 = MapPool::<()> {
            pool: vec![
                MapInPool {
                    map: Map::Metro,
                    mode: GameMode::Rush,
                    extra: (),
                },
                MapInPool {
                    map: Map::Locker,
                    mode: GameMode::Rush,
                    extra: (),
                },
            ],
        };

        let p2 = MapPool::<()> {
            pool: vec![MapInPool {
                map: Map::Metro,
                mode: GameMode::Rush,
                extra: (),
            }],
        };

        let p_removal = MapPool::<()> {
            pool: vec![MapInPool {
                map: Map::Locker,
                mode: GameMode::Rush,
                extra: (),
            }],
        };

        assert_eq!(p_removal, MapPool::removals(&p1, &p2));
        assert_eq!(MapPool::default(), MapPool::additions(&p1, &p2)); // no additions
    }

    #[test]
    fn changes() {
        let p1 = MapPool {
            pool: vec![
                MapInPool {
                    map: Map::Shanghai,
                    mode: GameMode::Rush,
                    extra: Vehicles(true),
                },
                MapInPool {
                    map: Map::Locker,
                    mode: GameMode::Rush,
                    extra: Vehicles(true),
                },
            ],
        };

        let p2 = MapPool {
            pool: vec![
                MapInPool {
                    map: Map::Shanghai,
                    mode: GameMode::Rush,
                    extra: Vehicles(false), // vehicles got disabled here
                },
                MapInPool {
                    map: Map::Locker,
                    mode: GameMode::Rush,
                    extra: Vehicles(true),
                },
            ],
        };

        let changes: Vec<(_, _)> = MapPool::changes(&p1, &p2)
            .iter()
            .map(|x| (x.0.to_owned(), x.1.to_owned()))
            .collect();
        assert_eq!(1, changes.len());
        assert_eq!(
            vec![(
                MapInPool {
                    map: Map::Shanghai,
                    mode: GameMode::Rush,
                    extra: Vehicles(false),
                },
                Vehicles(true)
            )],
            changes
        );
    }
}
