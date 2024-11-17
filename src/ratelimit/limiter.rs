use crate::ratelimit::bucket::RateLimitBucket;
use crate::ratelimit::error::RateLimited;
use std::hash::Hash;

#[derive(Debug)]
pub struct RateLimiter<K: Eq + Hash + Copy> {
    buckets: Vec<RateLimitBucket<K>>,
}

impl<K: Eq + Hash + Copy> RateLimiter<K> {
    pub fn new(buckets: Vec<RateLimitBucket<K>>) -> Self {
        Self { buckets }
    }

    pub async fn ratelimit(&self, key: K) -> Option<RateLimited> {
        let mut result = None;
        for bucket in &self.buckets {
            if let Some(limited) = bucket.ratelimit(key) {
                result = Some(limited);
            }
        }
        result
    }

    pub fn pump_limits(&self) {
        for bucket in &self.buckets {
            bucket.pump_limits();
        }
    }
}
