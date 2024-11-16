use crate::serialization::serializable::PacketSerializable;
use case_insensitive_hashmap::CaseInsensitiveHashMap;
use lazy_static::lazy_static;
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use std::str::FromStr;
use unicase::UniCase;

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

impl FromStr for ConnectionId {
    type Err = ParseConnectionIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let words: Vec<_> = s.split("-").collect();
        if words.len() != 3 {
            if words.len() != 1 {
                return Err(ParseConnectionIdError::incorrect_words(words.len()));
            }
            let word = words[0];
            if word.len() != 9 {
                return Err(ParseConnectionIdError::incorrect_short(word.len()));
            }
            return Ok(ConnectionId(u64::from_str_radix(word, 36)?));
        }
        let mut result = 0;
        let mut shift = 0;
        for word in words {
            let part = WORDS_FOR_CID_INVERSE
                .get(word)
                .ok_or_else(|| ParseConnectionIdError::unknown_word(String::from(word)))?;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseConnectionIdError {
    kind: ConnectionIdErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConnectionIdErrorKind {
    IncorrectWords(usize),
    IncorrectShort(usize),
    InvalidNumber(ParseIntError),
    UnknownWord(String),
}

impl ParseConnectionIdError {
    fn incorrect_words(words: usize) -> ParseConnectionIdError {
        ParseConnectionIdError {
            kind: ConnectionIdErrorKind::IncorrectWords(words),
        }
    }

    fn incorrect_short(length: usize) -> ParseConnectionIdError {
        ParseConnectionIdError {
            kind: ConnectionIdErrorKind::IncorrectShort(length),
        }
    }

    fn unknown_word(word: String) -> ParseConnectionIdError {
        ParseConnectionIdError {
            kind: ConnectionIdErrorKind::UnknownWord(word),
        }
    }
}

impl From<ParseIntError> for ParseConnectionIdError {
    fn from(value: ParseIntError) -> Self {
        ParseConnectionIdError {
            kind: ConnectionIdErrorKind::InvalidNumber(value),
        }
    }
}

impl Display for ParseConnectionIdError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ConnectionIdErrorKind::*;
        match &self.kind {
            IncorrectWords(words) => f.write_fmt(format_args!(
                "Three words are expected. Found {words} words."
            )),
            IncorrectShort(length) => f.write_fmt(format_args!(
                "Expected nine digit short connection ID, found {length} digits."
            )),
            InvalidNumber(error) => error.fmt(f),
            UnknownWord(word) => f.write_fmt(format_args!("Unknown word {word}.")),
        }
    }
}

impl Error for ParseConnectionIdError {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        use ConnectionIdErrorKind::*;
        match &self.kind {
            IncorrectWords(_) => "Three words are expected.",
            IncorrectShort(_) => "Expected nine digit short connection ID",
            InvalidNumber(err) => err.description(),
            UnknownWord(_) => "Unknown word.",
        }
    }
}
