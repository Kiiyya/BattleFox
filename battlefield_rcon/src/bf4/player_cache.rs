use super::Eaid;
use ascii::{AsciiStr, AsciiString};
use std::{collections::HashMap, time::Duration};
use tokio::time::Instant;

#[derive(Debug, Copy, Clone)]
pub(crate) struct PlayerCacheEntry {
    freshness: Instant,
    pub(crate) eaid: Eaid,
}

#[derive(Debug)]
pub(crate) struct PlayerEaidCache {
    cache: HashMap<AsciiString, PlayerCacheEntry>,
    last_clean: Instant,
    inserts_since_last_clean: usize,
}

const YEET_TIME: Duration = Duration::from_secs(60 * 3);

impl PlayerEaidCache {
    pub(crate) fn new() -> Self {
        Self {
            cache: HashMap::new(),
            last_clean: Instant::now(),
            inserts_since_last_clean: 0,
        }
    }

    pub(crate) fn try_get(&mut self, name: &AsciiStr) -> Option<PlayerCacheEntry> {
        let entry: Option<PlayerCacheEntry> = {
            if let Some(hit) = self.cache.get(name) {
                if hit.freshness.elapsed() > YEET_TIME {
                    // entries older than a few minutes are yeeted.
                    // self.cache.remove(name);
                    self.trim_if_needed(); // might as well use the occasion to clean up.
                    None
                } else {
                    Some(*hit)
                }
            } else {
                None
            }
            // we drop the mutex guard (`cache`) here.
        };
        entry
    }

    pub(crate) fn insert(&mut self, name: &AsciiStr, eaid: &Eaid) {
        self.trim_if_needed();
        self.cache.insert(
            name.to_owned(),
            PlayerCacheEntry {
                freshness: Instant::now(),
                eaid: *eaid,
            },
        );
        self.inserts_since_last_clean += 1;
    }

    pub(crate) fn trim_if_needed(&mut self) {
        if self.inserts_since_last_clean > 20 || self.last_clean.elapsed() >= YEET_TIME {
            // random number i pulled out of my butt.
            self.trim();
        }
    }

    pub(crate) fn trim(&mut self) {
        self.last_clean = Instant::now();
        self.inserts_since_last_clean = 0;

        // throw out any entries older than ...
        self.cache.retain(|_, v| v.freshness.elapsed() < YEET_TIME);
    }
}
