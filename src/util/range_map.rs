use log::error;
use std::fmt::Debug;

pub struct RangeMap<K: Copy + Debug + Ord, V: Copy> {
    key: Vec<K>,
    value: Vec<V>,
    len: usize,
}

impl<K: Copy + Debug + Ord, V: Copy> RangeMap<K, V> {
    pub fn new() -> Self {
        Self {
            key: Vec::new(),
            value: Vec::new(),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn shrink_to_fit(&mut self) {
        self.key.shrink_to_fit();
        self.value.shrink_to_fit();
    }

    pub fn put(&mut self, min: K, max: K, value: V) {
        if self.len > 0 {
            let key_index = self.len << 1;
            let prev_index = key_index - 2;
            let prev_max = self.key[prev_index + 1];
            if min <= prev_max {
                let prev_min = self.key[prev_index];
                error!(
                    "Range {min:?}..={max:?} isn't greater than previous max range {prev_min:?}..={prev_max:?}"
                );
                return;
            }
        }
        self.key.extend_from_slice(&[min, max]);
        self.value.push(value);
        self.len += 1;
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let index = self.key.binary_search(key).unwrap_or_else(|e| e);
        if (index & 1) == 1 || (index < (self.len << 1) && self.key[index] == *key) {
            Some(self.value[index >> 1])
        } else {
            None
        }
    }
}

pub type U32ToU32RangeMap = RangeMap<u32, u32>;
pub type U128ToU32RangeMap = RangeMap<u128, u32>;
