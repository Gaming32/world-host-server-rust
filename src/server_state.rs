use crate::connection::connection_id::ConnectionId;
use crate::connection::connection_set::ConnectionSet;
use crate::json_data::ExternalProxy;
use crate::modules::analytics::run_analytics;
use crate::modules::main_server::run_main_server;
use crate::modules::proxy_server::run_proxy_server;
use crate::modules::signalling_server::run_signalling_server;
use crate::protocol::port_lookup::ActivePortLookup;
use crate::SERVER_VERSION;
use linked_hash_set::LinkedHashSet;
use log::{info, warn};
use queues::Queue;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::Instant;
use try_catch::catch;
use uuid::Uuid;

#[derive(Debug)]
pub struct FullServerConfig {
    pub port: u16,
    pub base_addr: Option<String>,
    pub in_java_port: u16,
    pub ex_java_port: u16,
    pub analytics_time: Duration,
    pub external_servers: Option<Vec<Arc<ExternalProxy>>>,
}

pub struct ServerState {
    pub config: FullServerConfig,

    pub connections: Mutex<ConnectionSet>,

    pub proxy_connections: Mutex<HashMap<u64, (ConnectionId, Mutex<OwnedWriteHalf>)>>,

    pub remembered_friend_requests: Mutex<HashMap<Uuid, LinkedHashSet<Uuid>>>,
    pub received_friend_requests: Mutex<HashMap<Uuid, LinkedHashSet<Uuid>>>,

    pub port_lookups: Mutex<HashMap<Uuid, ActivePortLookup>>,
    pub port_lookup_by_expiry: Mutex<Queue<(Instant, ActivePortLookup)>>,
}

impl ServerState {
    pub fn new(config: FullServerConfig) -> Self {
        Self {
            config,

            connections: Mutex::new(ConnectionSet::new()),

            proxy_connections: Mutex::new(HashMap::new()),

            remembered_friend_requests: Mutex::new(HashMap::new()),
            received_friend_requests: Mutex::new(HashMap::new()),

            port_lookups: Mutex::new(HashMap::new()),
            port_lookup_by_expiry: Mutex::new(Queue::new()),
        }
    }

    pub async fn run(self) {
        info!(
            "Starting world-host-server {SERVER_VERSION} with {:?}",
            self.config
        );

        self.ping_external_servers();

        let state = Arc::new(self);

        macro_rules! run_sub_server {
            ($function:ident) => {{
                let state = state.clone();
                tokio::spawn(async move {
                    $function(state).await;
                });
            }};
        }

        run_sub_server!(run_analytics);
        run_sub_server!(run_proxy_server);
        run_sub_server!(run_signalling_server);
        run_main_server(state).await;
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
