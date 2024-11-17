use crate::country_code::CountryCode;
use crate::lat_long::LatitudeLongitude;

pub struct IpInfo {
    pub country: CountryCode,
    pub lat_long: LatitudeLongitude,
}

impl IpInfo {
    pub fn from_u32(x: u32) -> Self {
        Self {
            country: int_to_country(x & COUNTRY_MASK),
            lat_long: fixed22_to_lat_long(x >> LAT_LONG_SHIFT),
        }
    }

    pub fn to_u32(&self) -> u32 {
        let lat_long = lat_long_to_fixed22(self.lat_long);
        let country = country_to_int(self.country);
        (lat_long << LAT_LONG_SHIFT) | country
    }
}

const FIXED_11_SHIFT: u32 = 11;
const FIXED_11_MAGNITUDE: f64 = (1 << FIXED_11_SHIFT) as f64;
const FIXED_11_MASK: u32 = (1 << FIXED_11_SHIFT) - 1;
const COUNTRY_CHAR_BASE: u32 = 'A' as u32;
const COUNTRY_CHAR_SHIFT: u32 = 5;
const COUNTRY_CHAR_MASK: u32 = (1 << COUNTRY_CHAR_SHIFT) - 1;
const LAT_LONG_SHIFT: u32 = COUNTRY_CHAR_SHIFT * 2;
const COUNTRY_MASK: u32 = (1 << LAT_LONG_SHIFT) - 1;

fn fixed11_to_double(fixed: u32) -> f64 {
    (fixed as f64 * 360.0 / FIXED_11_MAGNITUDE) - 180.0
}

fn double_to_fixed11(double: f64) -> u32 {
    ((double + 180.0) / 360.0 * FIXED_11_MAGNITUDE) as u32
}

fn fixed22_to_lat_long(fixed: u32) -> LatitudeLongitude {
    let lat = fixed11_to_double((fixed >> FIXED_11_SHIFT) & FIXED_11_MASK);
    let long = fixed11_to_double(fixed & FIXED_11_MASK);
    LatitudeLongitude(lat, long)
}

fn lat_long_to_fixed22(lat_long: LatitudeLongitude) -> u32 {
    let lat = double_to_fixed11(lat_long.0);
    let long = double_to_fixed11(lat_long.1);
    (lat << FIXED_11_SHIFT) | long
}

fn country_char_to_int(char: u8) -> u32 {
    char as u32 - COUNTRY_CHAR_BASE
}

fn country_int_to_char(int: u32) -> char {
    char::from_u32(int + COUNTRY_CHAR_BASE).unwrap()
}

fn country_to_int(country: CountryCode) -> u32 {
    let chars = country.code();
    (country_char_to_int(chars[0]) << COUNTRY_CHAR_SHIFT) | country_char_to_int(chars[1])
}

fn int_to_country(int: u32) -> CountryCode {
    let char1 = country_int_to_char((int >> COUNTRY_CHAR_SHIFT) & COUNTRY_CHAR_MASK);
    let char2 = country_int_to_char(int & COUNTRY_CHAR_MASK);
    CountryCode::new(char1, char2).unwrap()
}
