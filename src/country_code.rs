use anyhow::bail;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct CountryCode {
    code: [u8; 2],
}

impl CountryCode {
    pub fn new(a: char, b: char) -> anyhow::Result<Self> {
        Ok(Self {
            code: [Self::validate(a)?, Self::validate(b)?],
        })
    }

    fn validate(c: char) -> anyhow::Result<u8> {
        if c.is_ascii_lowercase() {
            Ok(c as u8)
        } else {
            bail!("Invalid ISO alpha-2 character: {c}")
        }
    }

    pub fn code(&self) -> [u8; 2] {
        self.code
    }
}

impl FromStr for CountryCode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        if s.len() != 2 {
            bail!("ISO alpha-2 country code must be 2 digits");
        }
        let bytes = s.as_bytes();
        Ok(Self {
            code: [validate_u8_char(bytes[0])?, validate_u8_char(bytes[1])?],
        })
    }
}

fn validate_u8_char(c: u8) -> anyhow::Result<u8> {
    if c.is_ascii_uppercase() {
        Ok(c)
    } else {
        bail!("Invalid ISO alpha-2 character: {c}")
    }
}

impl Display for CountryCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}{}",
            self.code[0] as char, self.code[1] as char
        ))
    }
}

impl Debug for CountryCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Serialize for CountryCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for CountryCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(Error::custom)
    }
}
