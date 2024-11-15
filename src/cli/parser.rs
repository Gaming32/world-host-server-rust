use clap::builder::{StringValueParser, TypedValueParser};
use clap::error::ErrorKind::Format;
use clap::{Arg, Command, Error};
use parse_duration::parse;
use std::ffi::OsStr;
use std::time::Duration;

#[derive(Clone)]
pub struct DurationValueParser;

impl TypedValueParser for DurationValueParser {
    type Value = Duration;

    fn parse_ref(
        &self,
        cmd: &Command,
        arg: Option<&Arg>,
        value: &OsStr,
    ) -> Result<Self::Value, Error> {
        StringValueParser::new()
            .parse_ref(cmd, arg, value)
            .and_then(|value| parse(&value).map_err(|message| Error::raw(Format, message)))
    }
}
