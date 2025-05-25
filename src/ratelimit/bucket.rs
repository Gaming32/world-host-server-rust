use crate::ratelimit::error::RateLimited;
use dashmap::{DashMap, Entry};
use std::hash::Hash;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct RateLimitBucket<K: Eq + Hash + Copy> {
    name: String,
    max_count: u32,
    expiry: Duration,
    entries: DashMap<K, RateLimitEntry>,
}

#[derive(Copy, Clone, Debug)]
struct RateLimitEntry {
    time: Instant,
    count: u32,
}

impl<K: Eq + Hash + Copy> RateLimitBucket<K> {
    pub fn new(name: String, max_count: u32, expiry: Duration) -> Self {
        Self {
            name,
            max_count,
            expiry,
            entries: DashMap::new(),
        }
    }

    pub fn ratelimit(&self, key: K) -> Option<RateLimited> {
        let current_time = Instant::now();
        let entry = self.entries.entry(key);
        let value = match &entry {
            Entry::Occupied(v) => Some(v.get()),
            Entry::Vacant(_) => None,
        };
        if value.is_none() || current_time - value.unwrap().time >= self.expiry {
            entry.insert(RateLimitEntry {
                time: current_time,
                count: 1,
            });
            return None;
        }
        let value = *value.unwrap();
        if value.count < self.max_count {
            entry.insert(RateLimitEntry {
                time: current_time,
                count: value.count + 1,
            });
            return None;
        }
        drop(entry);
        Some(RateLimited::new(
            self.name.to_string(),
            value.time - current_time + self.expiry,
        ))
    }

    pub(super) fn pump_limits(&self) {
        self.entries.retain(|_, entry| entry.time.elapsed() < self.expiry)
    }
}
