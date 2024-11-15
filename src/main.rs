mod cli;
mod json_data;
mod lat_long;

use crate::cli::args::Args;
use crate::json_data::ExternalProxy;
use clap::Parser;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::process::exit;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut base_addr = args.base_addr;

    let external_servers = read_external_servers().unwrap_or_else(|error| {
        eprintln!("Error parsing external_proxies.json: {}", error);
        exit(1);
    });
    if let Some(servers) = external_servers {
        if servers.iter().filter(|s| s.addr.is_none()).count() > 1 {
            eprintln!("external_proxies.json defines must have no more than one missing addr field.");
            exit(1);
        }
        for server in servers {
            if server.addr.is_none() && server.base_addr.is_some() {
                if base_addr.is_none() {
                    base_addr = server.base_addr;
                } else {
                    println!("Both the CLI and external_proxies.json specify baseAddr for the local server.");
                    println!("--base-addr from the CLI will override the value in external_proxies.json.");
                }
                break;
            }
        }
    }

    if let Some(shutdown_time) = args.shutdown_time {
        tokio::spawn(async move {
            println!("Automatically shutting down after {:?}", shutdown_time);
            sleep(shutdown_time).await;
            println!("Shutting down because shutdown_time ({:?}) was reached", shutdown_time);
            exit(0);
        });
    }
}

fn read_external_servers() -> std::io::Result<Option<Vec<ExternalProxy>>> {
    let path = "external_proxies.json";
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
