pub mod ip_info;
pub mod ip_info_map;
pub mod java_util;
pub mod range_map;

pub fn copy_to_fixed_size<T: Default + Copy, const N: usize>(data: &[T]) -> [T; N] {
    let mut result = [T::default(); N];
    result.copy_from_slice(data);
    result
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
