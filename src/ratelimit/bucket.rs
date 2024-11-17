use crate::ratelimit::error::RateLimited;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct RateLimitBucket<K: Eq + Hash + Copy> {
    name: String,
    max_count: u32,
    expiry: Duration,
    entries: Mutex<HashMap<K, RateLimitEntry>>,
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
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn ratelimit(&self, key: K) -> Option<RateLimited> {
        let mut entries = self.entries.lock().unwrap();
        let entry = entries.get(&key);
        let current_time = Instant::now();
        if entry.is_none() || current_time - entry.unwrap().time >= self.expiry {
            entries.insert(
                key,
                RateLimitEntry {
                    time: current_time,
                    count: 1,
                },
            );
            return None;
        }
        let entry = *entry.unwrap();
        if entry.count < self.max_count {
            entries.insert(
                key,
                RateLimitEntry {
                    time: current_time,
                    count: entry.count + 1,
                },
            );
            return None;
        }
        Some(RateLimited::new(
            self.name.to_string(),
            entry.time - current_time + self.expiry,
        ))
    }

    pub(super) fn pump_limits(&self) {
        self.entries
            .lock()
            .unwrap()
            .retain(|_, entry| Instant::now() - entry.time < self.expiry)
    }
}
