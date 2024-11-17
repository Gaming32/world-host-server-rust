mod authlib;
mod cli;
mod connection;
mod country_code;
mod json_data;
mod lat_long;
mod logging;
mod minecraft_crypt;
mod modules;
mod protocol;
mod ratelimit;
mod serialization;
mod server_state;
mod socket_wrapper;
mod util;

use crate::cli::args::Args;
use crate::json_data::ExternalProxy;
use crate::server_state::{FullServerConfig, ServerState};
use clap::Parser;
use log::{error, info};
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process::exit;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::sleep;

pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), '/', env!("CARGO_PKG_VERSION"));

fn main() {
    let args = Args::parse();
    logging::init_logging(args.log_config);
    let mut base_addr = args.base_addr;

    let external_servers = read_external_servers().unwrap_or_else(|error| {
        error!("Error parsing external_proxies.json: {error}");
        exit(1);
    });
    if let Some(servers) = &external_servers {
        if servers.iter().filter(|s| s.addr.is_none()).count() > 1 {
            error!("external_proxies.json defines must have no more than one missing addr field.");
            exit(1);
        }
        for server in servers {
            if server.addr.is_none() && server.base_addr.is_some() {
                if base_addr.is_none() {
                    base_addr = server.base_addr.clone();
                } else {
                    info!("Both the CLI and external_proxies.json specify baseAddr for the local server.");
                    info!("--base-addr from the CLI will override the value in external_proxies.json.");
                }
                break;
            }
        }
    }

    if let Some(shutdown_time) = args.shutdown_time {
        tokio::spawn(async move {
            info!("Automatically shutting down after {shutdown_time:?}");
            sleep(shutdown_time).await;
            info!("Shutting down because shutdown_time ({shutdown_time:?}) was reached");
            exit(0);
        });
    }

    tokio::runtime::Runtime::new().unwrap().block_on(async {
        ServerState::new(FullServerConfig {
            port: args.port,
            base_addr,
            in_java_port: args.in_java_port,
            ex_java_port: args.ex_java_port.unwrap_or(args.in_java_port),
            analytics_time: args.analytics_time,
            external_servers,
        })
        .run()
        .await;
    });
}

pub fn init_logging(log_config: Option<String>) {
    if let Some(config_path) = log_config {
        log4rs::init_file(config_path.clone(), Default::default()).unwrap_or_else(|error| {
            eprintln!("Failed to load log config {config_path}: {error}");
            exit(1);
        });
    } else {
        let config = include_str!("default_logging.yml");
        let config = serde_yaml::from_str::<log4rs::config::RawConfig>(config).unwrap();
        log4rs::init_raw_config(config).unwrap();
    }
}

fn read_external_servers() -> std::io::Result<Option<Vec<ExternalProxy>>> {
    let path = Path::new("external_proxies.json");
    if !fs::exists(path)? {
        return Ok(None);
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    Ok(serde_json::from_reader(reader)?)
}

#[allow(dead_code)]
async fn old_main() {
    let listener = TcpListener::bind("0.0.0.0:1234").await.unwrap();

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        println!("Received connection from {:?}", socket.peer_addr());
        tokio::spawn(async move {
            process(socket).await;
        });
    }
}

async fn process(mut socket: TcpStream) {
    while let Ok(byte) = socket.read_i8().await {
        socket.write_i8(byte).await.unwrap();
    }
}
