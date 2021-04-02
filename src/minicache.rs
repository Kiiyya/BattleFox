use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Mutex, MutexGuard},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use futures::{future::BoxFuture, Future};

use crate::guard::{Guard, Judgement, recent::Recent};

#[async_trait]
pub trait Resolver<K, V> {
    /// Error type.
    type E;

    fn resolve(&self, key: &K) -> Result<V, Self::E>;
}

#[async_trait]
pub trait Mached2<T, J: Judgement<T>> {
    async fn get<R: Resolver<T>>(&self, resolver: &R) -> Guard<T, Recent<J>>;
}

// #[async_trait]
// pub trait Mached<T> {
//     async fn get<R: Resolver<T>>(&self, resolver: &R) -> &T;
//     async fn get_mut<R: Resolver<T>>(&mut self, resolver: &R) -> &mut T;
//     fn try_get(&self) -> Option<&T>;
//     fn try_get_mut(&mut self) -> Option<&mut T>;
// }

pub struct Mached<V> {
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
