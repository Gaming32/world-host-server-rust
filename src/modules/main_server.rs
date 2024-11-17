use crate::authlib::auth_service::YggdrasilAuthenticationService;
use crate::connection::connection_id::ConnectionId;
use crate::connection::{Connection, LiveConnection};
use crate::minecraft_crypt;
use crate::minecraft_crypt::RsaKeyPair;
use crate::protocol::protocol_versions;
use crate::protocol::s2c_message::WorldHostS2CMessage;
use crate::ratelimit::bucket::RateLimitBucket;
use crate::ratelimit::limiter::RateLimiter;
use crate::server_state::ServerState;
use crate::socket_wrapper::SocketWrapper;
use crate::util::ext::WHAsyncReadExt;
use crate::util::ip_info_map::IpInfoMap;
use log::{error, info, warn};
use std::net::IpAddr;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::time::{interval_at, Instant, MissedTickBehavior};
use uuid::Uuid;

pub async fn run_main_server(server: Arc<ServerState>) {
    let session_service = YggdrasilAuthenticationService::new().create_session_service();
    let ip_info_map = load_ip_info_map().await;

    info!("Generating key pair");
    let key_pair = Arc::new(minecraft_crypt::generate_key_pair());

    info!("Staring World Host server on port {}", server.config.port);
    let rate_limiter = Arc::new(RateLimiter::<IpAddr>::new(vec![
        RateLimitBucket::new("per_minute".to_string(), 20, Duration::from_secs(60)),
        RateLimitBucket::new("per_hour".to_string(), 400, Duration::from_secs(60 * 60)),
    ]));
    {
        let rate_limiter = rate_limiter.clone();
        tokio::spawn(async move {
            const PUMP_TIME: Duration = Duration::from_secs(60);
            let mut interval = interval_at(Instant::now() + PUMP_TIME, PUMP_TIME);
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
            loop {
                interval.tick().await;
                let rate_limiter = rate_limiter.clone();
                tokio::task::spawn_blocking(move || rate_limiter.pump_limits())
                    .await
                    .unwrap();
            }
        });
    }

    let listener = TcpListener::bind(("0.0.0.0", server.config.port))
        .await
        .unwrap_or_else(|error| {
            error!("Failed to bind: {error}");
            exit(1);
        });
    info!(
        "Started World Host server on {}",
        listener.local_addr().unwrap()
    );

    loop {
        let result = listener.accept().await;
        if let Err(error) = result {
            error!("Failed to accept connection: {error}");
            continue;
        }
        let (socket, addr) = result.unwrap();
        if let Err(error) = socket2::SockRef::from(&socket).set_keepalive(true) {
            warn!("Failed to set SO_KEEPALIVE on socket for {addr}: {error}");
        }

        let rate_limiter = rate_limiter.clone();
        let server = server.clone();
        let key_pair = key_pair.clone();
        tokio::spawn(async move {
            let mut socket = SocketWrapper(socket);
            if let Some(limited) = rate_limiter.ratelimit(addr.ip()).await {
                warn!("{} is reconnecting too quickly! {limited}", addr.ip());
                socket
                    .send_close_error(format!("Ratelimit exceeded! {limited}"))
                    .await;
                return;
            }

            let mut connection = None;
            if let Err(error) =
                handle_connection(&key_pair, socket, addr.ip(), &mut connection).await
            {
                info!("Connection {} closed due to {error}", addr);
            }
            if let Some(connection) = connection {
                connection.live.lock().await.open = false;
                info!("Connection {} from {} closed", connection.id, addr);
                server.connections.lock().await.remove(&connection);
                // TODO: Broadcast ClosedWorld
                info!(
                    "There are {} open connections.",
                    server.connections.lock().await.len()
                );
            }
        });
    }
}

async fn load_ip_info_map() -> IpInfoMap {
    info!("Downloading IP info map...");
    let start = Instant::now();
    let result = IpInfoMap::load_from_compressed_geolite_city_files(
        if !cfg!(debug_assertions) { // This takes a whopping 15 seconds (on my computer) under the dev target!
            vec![
                "https://github.com/sapics/ip-location-db/raw/main/geolite2-city/geolite2-city-ipv4-num.csv.gz",
                "https://github.com/sapics/ip-location-db/raw/main/geolite2-city/geolite2-city-ipv6-num.csv.gz",
            ]
        } else {
            vec![]
        }
    ).await;
    let duration = start.elapsed();
    match result {
        Ok(map) => {
            info!(
                "Downloaded IP info map in {duration:?} ({} entries)",
                map.len()
            );
            map
        }
        Err(err) => {
            error!("Failed to download IP info map in {duration:?}: {err}");
            IpInfoMap::default()
        }
    }
}

async fn handle_connection(
    key_pair: &RsaKeyPair,
    mut socket: SocketWrapper,
    remote_addr: IpAddr,
    connection_out: &mut Option<Connection>,
) -> anyhow::Result<()> {
    let protocol_version = socket.0.read_u32().await;
    if protocol_version.is_err() {
        info!("Received a ping connection (immediate disconnect)");
        return Ok(());
    }
    let protocol_version = protocol_version?;

    if !protocol_versions::SUPPORTED.contains(&protocol_version) {
        socket
            .send_close_error(format!("Unsupported protocol version {protocol_version}"))
            .await;
        return Ok(());
    }

    let connection = match create_connection(socket, remote_addr, key_pair, protocol_version).await
    {
        Some(conn) => conn,
        None => {
            return Ok(());
        }
    };
    *connection_out = Some(connection);

    Ok(())
}

async fn create_connection(
    mut socket: SocketWrapper,
    remote_addr: IpAddr,
    key_pair: &RsaKeyPair,
    protocol_version: u32,
) -> Option<Connection> {
    let handshake_result =
        perform_versioned_handshake(&mut socket, key_pair, protocol_version).await;
    if let Err(error) = handshake_result {
        warn!("Handshake from {remote_addr} failed: {error}");
        socket.send_close_error(error.to_string()).await;
        return None;
    }
    let handshake_result = handshake_result.unwrap();

    if let Some(warning) = handshake_result.warning {
        warn!("Warning in handshake from {remote_addr}: {warning}");
        if let Err(error) = socket
            .send_message(WorldHostS2CMessage::Warning {
                message: warning,
                important: false,
            })
            .await
        {
            error!("Failed to send warning to {remote_addr}: {error}");
            return None;
        }
    }
    Some(Connection {
        id: handshake_result.connection_id,
        addr: remote_addr,
        user_uuid: handshake_result.user_id,
        live: Arc::new(Mutex::new(LiveConnection {
            socket,
            country: None,
            open: true,
        })),
    })
}

async fn perform_versioned_handshake(
    socket: &mut SocketWrapper,
    key_pair: &RsaKeyPair,
    protocol_version: u32,
) -> anyhow::Result<HandshakeResult> {
    if protocol_version < protocol_versions::NEW_AUTH_PROTOCOL {
        Ok(HandshakeResult {
            user_id: socket.0.read_uuid().await?,
            connection_id: ConnectionId::new(socket.0.read_u64().await?)?,
            warning: None,
        })
    } else {
        perform_handshake(
            socket,
            key_pair,
            protocol_version >= protocol_versions::ENCRYPTED_PROTOCOL,
        )
        .await
    }
}

async fn perform_handshake(
    socket: &mut SocketWrapper,
    key_pair: &RsaKeyPair,
    supports_encryption: bool,
) -> anyhow::Result<HandshakeResult> {
    todo!("Modern handshake");
}

#[derive(Clone, Debug)]
struct HandshakeResult {
    user_id: Uuid,
    connection_id: ConnectionId,
    warning: Option<String>,
}
