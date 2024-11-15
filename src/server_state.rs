use std::time::Duration;
use log::{info, warn};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::sleep;
use try_catch::catch;
use crate::json_data::ExternalProxy;
use crate::modules::analytics::run_analytics;
use crate::SERVER_VERSION;

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
    config: FullServerConfig,
}

impl ServerState {
    pub fn new(config: FullServerConfig) -> Self {
        Self {
            config
        }
    }

    pub async fn run(self) {
        info!("Starting world-host-server {SERVER_VERSION} with {:?}", self.config);

        self.ping_external_servers();

        let analytics_time = self.config.analytics_time;
        tokio::spawn(async move {
            run_analytics(analytics_time).await;
        });

        sleep(Duration::from_secs(30)).await;
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
