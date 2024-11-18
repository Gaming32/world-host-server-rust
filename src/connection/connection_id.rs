use crate::serialization::serializable::PacketSerializable;
use anyhow::{anyhow, bail};
use case_insensitive_hashmap::CaseInsensitiveHashMap;
use lazy_static::lazy_static;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use unicase::UniCase;

const MAX_CONNECTION_IDS: u64 = 1 << 42;
const WORD_SHIFT: u8 = 14;
const WORD_MASK: u64 = (1 << WORD_SHIFT) - 1;

lazy_static! {
    static ref WORDS_FOR_CID: Vec<String> = include_str!("16k.txt")
        .lines()
        .filter(|line| !line.starts_with("//"))
        .map(String::from)
        .collect();
    static ref WORDS_FOR_CID_INVERSE: CaseInsensitiveHashMap<u16> = WORDS_FOR_CID
        .iter()
        .enumerate()
        .map(|(index, value)| (UniCase::new(value.clone()), index as u16))
        .collect();
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ConnectionId(u64);

impl ConnectionId {
    pub fn new(id: u64) -> anyhow::Result<Self> {
        if (0..MAX_CONNECTION_IDS).contains(&id) {
            Ok(ConnectionId(id))
        } else {
            bail!("Connection ID {id} out of range")
        }
    }
}

impl FromStr for ConnectionId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<ConnectionId> {
        let words: Vec<_> = s.split("-").collect();
        if words.len() != 3 {
            if words.len() != 1 {
                bail!("Three words are expected. Found {} words.", words.len());
            }
            let word = words[0];
            if word.len() != 9 {
                bail!(
                    "Expected nine digit short connection ID, found {} digits.",
                    word.len()
                );
            }
            return Ok(ConnectionId(u64::from_str_radix(word, 36)?));
        }
        let mut result = 0;
        let mut shift = 0;
        for word in words {
            let part = WORDS_FOR_CID_INVERSE
                .get(word)
                .ok_or_else(|| anyhow!("Unknown word {word}."))?;
            result |= (*part as u64) << shift;
            shift += WORD_SHIFT;
        }
        Ok(ConnectionId(result))
    }
}

impl Display for ConnectionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let first = (self.0 & WORD_MASK) as usize;
        let second = ((self.0 >> WORD_SHIFT) & WORD_MASK) as usize;
        let third = ((self.0 >> WORD_SHIFT >> WORD_SHIFT) & WORD_MASK) as usize;
        f.write_fmt(format_args!(
            "{}-{}-{}",
            WORDS_FOR_CID[first], WORDS_FOR_CID[second], WORDS_FOR_CID[third]
        ))
    }
}

impl PacketSerializable for ConnectionId {
    fn serialize_to(&self, buf: &mut Vec<u8>) {
        self.0.serialize_to(buf)
    }
}
