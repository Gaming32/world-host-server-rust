pub mod ip_info;
pub mod ip_info_map;
pub mod java_util;
pub mod range_map;

pub fn copy_to_fixed_size<T: Default + Copy, const N: usize>(data: &[T]) -> [T; N] {
    let mut result = [T::default(); N];
    result.copy_from_slice(data);
    result
}
