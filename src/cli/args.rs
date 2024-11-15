use crate::cli::parser::DurationValueParser;
use clap::Parser;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Port to bind to
    #[arg(short, long, default_value = "9646")]
    pub port: u16,

    /// Base address to use for proxy connections
    #[arg(short = 'a', long)]
    pub base_addr: Option<String>,

    /// Port to use for Java Edition proxy connections
    #[arg(short = 'j', long, default_value = "25565")]
    pub in_java_port: u16,

    /// External port to use for Java Edition proxy connections
    #[arg(short = 'J', long)]
    pub ex_java_port: Option<u16>,

    /// Amount of time between analytics syncs
    #[arg(long, default_value = "0m", value_parser = DurationValueParser)]
    pub analytics_time: Duration,
    
    /// The amount of time before the server automatically shuts down. Useful for restart scripts.
    #[arg(long, value_parser = DurationValueParser)]
    pub shutdown_time: Option<Duration>,
    
    /// The path to a log4rs yaml logging configuration
    #[arg(long)]
    pub log_config: Option<String>,
}
