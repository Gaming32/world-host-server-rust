use crate::connection::connection_set::ConnectionSet;
use crate::json_data::ExternalProxy;
use crate::modules::analytics::run_analytics;
use crate::modules::main_server::run_main_server;
use crate::SERVER_VERSION;
use log::{info, warn};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use try_catch::catch;

#[derive(Debug)]
pub struct FullServerConfig {
    pub port: u16,
    pub base_addr: Option<String>,
    pub in_java_port: u16,
    pub ex_java_port: u16,
    pub analytics_time: Duration,
    pub external_servers: Option<Vec<ExternalProxy>>,
}

pub struct ServerState {
    pub config: FullServerConfig,
    pub connections: Mutex<ConnectionSet>,
}

impl ServerState {
    pub fn new(config: FullServerConfig) -> Self {
        Self {
            config,
            connections: Mutex::new(ConnectionSet::new()),
        }
    }

    pub async fn run(self) {
        info!(
            "Starting world-host-server {SERVER_VERSION} with {:?}",
            self.config
        );

        self.ping_external_servers();

        let state = Arc::new(self);
        let cloned_state = state.clone();
        tokio::spawn(async move {
            run_analytics(cloned_state.as_ref()).await;
        });

        run_main_server(state.as_ref()).await;
    }

    fn ping_external_servers(&self) {
        if let Some(servers) = &self.config.external_servers {
            for proxy in servers {
                if let Some(proxy_addr) = &proxy.addr {
                    let proxy_addr = proxy_addr.clone();
                    let proxy_port = proxy.port;
                    tokio::spawn(async move {
                        let display_addr = format!("{proxy_addr}:{proxy_port}");
                        info!("Attempting to ping {display_addr}");
                        catch! {
                            try {
                                TcpStream::connect((proxy_addr, proxy_port)).await?.shutdown().await?;
                                info!("Successfully pinged {display_addr}");
                            } catch error {
                                warn!("Failed to ping {display_addr}: {error}");
                            }
                        }
                    });
                }
            }
        }
    }
}
