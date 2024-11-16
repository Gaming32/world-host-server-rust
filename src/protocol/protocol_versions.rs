use std::ops::RangeInclusive;

pub const CURRENT: u32 = 7;
pub const STABLE: u32 = 7;
pub const SUPPORTED: RangeInclusive<u32> = CURRENT..=STABLE;

pub const NEW_AUTH_PROTOCOL: u32 = 6;
pub const ENCRYPTED_PROTOCOL: u32 = 7;

pub fn get_version_name(protocol: u32) -> &'static str {
    match protocol {
        2 => "0.3.2",
        3 => "0.3.4",
        4 => "0.4.3",
        5 => "0.4.4",
        6 => "0.4.14",
        7 => "0.5.0",
        _ => panic!("Invalid protocol version {protocol}"),
    }
}
