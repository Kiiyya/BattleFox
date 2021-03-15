use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Mutex, MutexGuard},
    time::{Duration, Instant},
};

use futures::{future::BoxFuture, Future};

struct Line<V> {
    value: V,
    freshness: Instant,
}

pub struct MiniCache<K: Hash + Ord, V> {
    lines: Mutex<HashMap<K, Line<V>>>,
    max_size: usize,
    ttl: Duration,
}

impl<K: Hash + Ord, V> MiniCache<K, V> {
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            lines: Mutex::new(HashMap::new()),
            max_size,
            ttl,
        }
    }

    pub fn try_get(&self, key: &K) -> Option<&V> {
        let mut lock = self.lines.lock().unwrap();
        let line = lock.get(key);
        if let Some(line) = line {
            if line.freshness.elapsed() < self.ttl {
                lock.remove(key);
                None
            } else {
                Some(&line.value)
            }
        } else {
            None
        }
    }

    pub async fn get_or_resolve<
        E,
        Fut: Future<Output = Result<V, E>>,
        Resolver: FnOnce() -> Fut,
    >(
        &self,
        key: &K,
        resolver: Resolver,
    ) -> Result<&V, E>
    where
        K: Clone,
    {
        if let Some(hit) = self.try_get(key) {
            Ok(hit)
        } else {
            let result = resolver().await?;
            let mut lock = self.lines.lock().unwrap();
            self.insert_locked(&mut lock, key.clone(), result);

            if let Some(line) = lock.get(key) {
                // Ok(&line.value)
                todo!()
            } else {
                todo!()
            }
            // let line : &Line<V> = lock.get(key);
            // Ok(&line.value)
        }
    }

    // /// If value existed before, it is dropped.
    // pub fn insert(&self, key: K, value: V) {
    //     let mut lock = self.lines.lock().unwrap();
    //     Self::insert_locked(&mut lock)
    // }

    fn insert_locked(&self, lock: &mut HashMap<K, Line<V>>, key: K, value: V) {
        self.trim_if_needed_locked(&mut lock);
        lock.insert(
            key,
            Line {
                value,
                freshness: Instant::now(),
            },
        );
    }

    fn trim_if_needed_locked(&self, lock: &mut HashMap<K, Line<V>>) {}
}
