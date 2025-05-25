use dashmap::DashMap;
use linked_hash_set::LinkedHashSet;
use std::hash::Hash;

pub mod ip_info;
pub mod ip_info_map;
pub mod java_util;
pub mod mc_packet;
pub mod range_map;

pub fn copy_to_fixed_size<T: Default + Copy, const N: usize>(data: &[T]) -> [T; N] {
    let mut result = [T::default(); N];
    result.copy_from_slice(data);
    result
}

pub fn remove_double_key<A: Hash + Eq, B: Hash + Eq>(
    map: &DashMap<A, LinkedHashSet<B>>,
    a: &A,
    b: &B,
) {
    if let Some(mut sub) = map.get_mut(a) {
        sub.remove(b);
        if sub.is_empty() {
            map.remove(a);
        }
    }
}

pub fn add_with_circle_limit<Q: Hash + Eq>(
    set: &mut LinkedHashSet<Q>,
    key: Q,
    limit: usize,
) -> Option<Q> {
    if set.insert(key) {
        if set.len() > limit {
            set.pop_front()
        } else {
            None
        }
    } else {
        None
    }
}

// Like bail!, but for io::Result
#[macro_export]
macro_rules! invalid_data {
    ($msg:literal $(,)?) => {
        $crate::__invalid_data_impl!(format!($msg))
    };
    ($err:expr $(,)?) => {
        $crate::__invalid_data_impl!(format!($err))
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::__invalid_data_impl!(format!($fmt, $($arg)*))
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __invalid_data_impl {
    ($msg:expr) => {
        return std::io::Result::Err(std::io::Error::new(std::io::ErrorKind::InvalidData, $msg))
    };
}
