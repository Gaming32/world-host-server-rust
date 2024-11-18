use crate::authlib::auth_service::YggdrasilAuthenticationService;
use crate::authlib::session_service::YggdrasilMinecraftSessionService;
use crate::connection::connection_id::ConnectionId;
use crate::connection::{Connection, LiveConnection};
use crate::minecraft_crypt;
use crate::minecraft_crypt::{Aes128Cfb, RsaKeyPair};
use crate::protocol::data_ext::WHAsyncReadExt;
use crate::protocol::s2c_message::WorldHostS2CMessage;
use crate::protocol::security::SecurityLevel;
use crate::protocol::{message_handler, protocol_versions};
use crate::ratelimit::bucket::RateLimitBucket;
use crate::ratelimit::limiter::RateLimiter;
use crate::server_state::ServerState;
use crate::socket_wrapper::SocketWrapper;
use crate::util::ip_info_map::IpInfoMap;
use crate::util::java_util::java_name_uuid_from_bytes;
use crate::util::remove_double_key;
use log::{debug, error, info, warn};
use num_bigint::BigInt;
use rand::RngCore;
use rsa::pkcs8::EncodePublicKey;
use std::io;
use std::net::IpAddr;
use std::ops::DerefMut;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::yield_now;
use tokio::time::{interval_at, Instant, MissedTickBehavior};
use uuid::Uuid;

pub async fn run_main_server(server: Arc<ServerState>) {
    let session_service = YggdrasilAuthenticationService::new().create_session_service();
    let ip_info_map = load_ip_info_map().await;

    info!("Generating key pair");
    let key_pair = minecraft_crypt::generate_key_pair();

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

    let state = MainServerState {
        server,
        session_service: Arc::new(session_service),
        key_pair: Arc::new(key_pair),
        ip_info_map: Arc::new(ip_info_map),
    };
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
        let state = state.clone();
        tokio::spawn(async move {
            let mut socket = SocketWrapper(socket);
            if let Some(limited) = rate_limiter.ratelimit(addr.ip()).await {
                warn!("{} is reconnecting too quickly! {limited}", addr.ip());
                let message = format!("Ratelimit exceeded! {limited}");
                socket.close_error(message, &mut None).await;
                return;
            }

            let mut connection = None;
            if let Err(error) = handle_connection(&state, socket, addr.ip(), &mut connection).await
            {
                info!("Connection {} closed due to {error}", addr);
                if let Some(connection) = &connection {
                    connection.close_error(error.to_string()).await;
                }
            }
            if let Some(connection) = connection {
                connection.live.lock().await.open = false;
                info!("Connection {} from {} closed", connection.id, addr);
                state.server.connections.lock().await.remove(&connection);
                // TODO: Broadcast ClosedWorld
                info!(
                    "There are {} open connections.",
                    state.server.connections.lock().await.len()
                );
            }
        });
    }
}

#[derive(Clone)]
struct MainServerState {
    server: Arc<ServerState>,
    session_service: Arc<YggdrasilMinecraftSessionService>,
    key_pair: Arc<RsaKeyPair>,
    ip_info_map: Arc<IpInfoMap>,
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
    state: &MainServerState,
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
        let message = format!("Unsupported protocol version {protocol_version}");
        socket.close_error(message, &mut None).await;
        return Ok(());
    }

    let connection = match create_connection(socket, remote_addr, state, protocol_version).await {
        Some(conn) => conn,
        None => {
            return Ok(());
        }
    };
    *connection_out = Some(connection.clone());

    info!(
        "Connection opened: {} ({}) from {}",
        connection.id, connection.user_uuid, connection.addr
    );

    let latest_visible_protocol_version = if protocol_version <= protocol_versions::STABLE {
        protocol_versions::STABLE
    } else {
        protocol_versions::CURRENT
    };
    connection
        .send_message(&WorldHostS2CMessage::ConnectionInfo {
            connection_id: connection.id,
            base_ip: state.server.config.base_addr.clone().unwrap_or_default(),
            base_port: state.server.config.ex_java_port,
            user_ip: remote_addr.to_string(),
            protocol_version: latest_visible_protocol_version,
            punch_port: 0,
        })
        .await?;
    if protocol_version < latest_visible_protocol_version {
        warn!(
            "Client {} has an outdated client! Client version: {}. Server version: {} (stable {})",
            connection.id,
            protocol_version,
            protocol_versions::CURRENT,
            protocol_versions::STABLE
        );
        connection
            .send_message(&WorldHostS2CMessage::OutdatedWorldHost {
                recommended_version: protocol_versions::get_version_name(
                    latest_visible_protocol_version,
                )
                .to_string(),
            })
            .await?
    }

    if connection.security_level() == SecurityLevel::Insecure
        && connection.user_uuid.get_version_num() == 4
    {
        // Using Error because Warning was added in the same protocol version that Secure was
        connection.send_message(&WorldHostS2CMessage::Error {
            message: format!("You are using an old insecure version of World Host. It is highly recommended that you update to {} or later.", protocol_versions::get_version_name(protocol_versions::NEW_AUTH_PROTOCOL)),
            critical: false,
        }).await?;
    }

    if let Some(ip_info) = state.ip_info_map.get(remote_addr) {
        connection.live.lock().await.country = Some(ip_info.country);
        if let Some(external_servers) = &state.server.config.external_servers {
            if let Some(proxy) = external_servers.iter().min_by(|a, b| {
                f64::total_cmp(
                    &a.lat_long.haversine_distance(&ip_info.lat_long),
                    &b.lat_long.haversine_distance(&ip_info.lat_long),
                )
            }) {
                if let Some(addr) = &proxy.addr {
                    connection.live.lock().await.external_proxy = Some(proxy.clone());
                    connection
                        .send_message(&WorldHostS2CMessage::ExternalProxyServer {
                            host: addr.clone(),
                            port: proxy.port,
                            base_addr: proxy.base_addr.clone().unwrap_or_else(|| addr.clone()),
                            mc_port: proxy.mc_port,
                        })
                        .await?;
                }
            }
        }
    }

    {
        let start = Instant::now();
        let connections = &state.server.connections;
        while !connections.lock().await.add(connection.clone()) {
            {
                let mut connections = connections.lock().await;
                let other = connections.by_id(connection.id);
                if let Some(other) = other {
                    if other.addr == connection.addr {
                        other
                            .close_error("Connection ID taken by same IP".to_string())
                            .await;
                        connections.add_force(connection.clone());
                        break;
                    }
                }
            }
            if Instant::now() - start > Duration::from_millis(500) {
                warn!(
                    "ID {} used twice. Disconnecting new connection.",
                    connection.id
                );
                connection
                    .close_error("That connection ID is taken.".to_string())
                    .await;
                return Ok(());
            }
            yield_now().await;
        }
    }

    info!(
        "There are {} open connections",
        state.server.connections.lock().await.len()
    );

    dequeue_friend_requests(&connection, &state.server).await?;

    loop {
        let message = connection.recv_message().await;
        if message.is_err() {
            return Ok(());
        }
        let message = message?;
        debug!("Received message {message:?}");
        if let Err(error) =
            message_handler::handle_message(message, &connection, state.server.as_ref()).await
        {
            error!("A critical error occurred in client handling: {error}");
            connection.close_error(error.to_string()).await;
            return Err(error);
        }
    }
}

async fn dequeue_friend_requests(connection: &Connection, server: &ServerState) -> io::Result<()> {
    let received = server
        .received_friend_requests
        .lock()
        .await
        .remove(&connection.user_uuid);
    if received.is_none() {
        return Ok(());
    }
    let received = received.unwrap();
    let mut remembered = server.remembered_friend_requests.lock().await;
    for received_from in received {
        connection
            .send_message(&WorldHostS2CMessage::FriendRequest {
                from_user: received_from,
                security: SecurityLevel::from(received_from, true),
            })
            .await?;
        remove_double_key(
            remembered.deref_mut(),
            &received_from,
            &connection.user_uuid,
        );
    }
    Ok(())
}

async fn create_connection(
    mut socket: SocketWrapper,
    remote_addr: IpAddr,
    state: &MainServerState,
    protocol_version: u32,
) -> Option<Connection> {
    let handshake_result = perform_versioned_handshake(&mut socket, state, protocol_version).await;
    if let Err(error) = handshake_result {
        warn!("Failed to perform handshake from {remote_addr}: {error}");
        let message = error.to_string();
        socket.close_error(message, &mut None).await;
        return None;
    }
    let handshake_result = handshake_result.unwrap();
    let mut encrypt_cipher = handshake_result.encrypt_cipher;

    if handshake_result.success {
        if let Some(warning) = handshake_result.message {
            warn!("Warning in handshake from {remote_addr}: {warning}");
            if let Err(error) = socket
                .send_message(
                    &WorldHostS2CMessage::Warning {
                        message: warning,
                        important: false,
                    },
                    &mut encrypt_cipher,
                )
                .await
            {
                error!("Failed to send warning to {remote_addr}: {error}");
                return None;
            }
        }
    } else {
        let message = handshake_result.message.unwrap();
        warn!("Handshake from {remote_addr} failed: {message}");
        socket.close_error(message, &mut encrypt_cipher).await;
        return None;
    }

    Some(Connection {
        id: handshake_result.connection_id,
        addr: remote_addr,
        user_uuid: handshake_result.user_id,
        protocol_version,
        live: Arc::new(Mutex::new(LiveConnection {
            socket,
            country: None,
            external_proxy: None,
            open: true,
            encrypt_cipher,
            decrypt_cipher: handshake_result.decrypt_cipher,
        })),
    })
}

async fn perform_versioned_handshake(
    socket: &mut SocketWrapper,
    state: &MainServerState,
    protocol_version: u32,
) -> anyhow::Result<HandshakeResult> {
    if protocol_version < protocol_versions::NEW_AUTH_PROTOCOL {
        Ok(HandshakeResult {
            user_id: socket.0.read_uuid().await?,
            connection_id: ConnectionId::new(socket.0.read_u64().await?)?,
            encrypt_cipher: None,
            decrypt_cipher: None,
            success: true,
            message: None,
        })
    } else {
        perform_handshake(
            socket,
            state,
            protocol_version >= protocol_versions::ENCRYPTED_PROTOCOL,
        )
        .await
    }
}

struct HandshakeResult {
    user_id: Uuid,
    connection_id: ConnectionId,
    encrypt_cipher: Option<Aes128Cfb>,
    decrypt_cipher: Option<Aes128Cfb>,
    success: bool,
    message: Option<String>,
}

async fn perform_handshake(
    socket: &mut SocketWrapper,
    state: &MainServerState,
    supports_encryption: bool,
) -> anyhow::Result<HandshakeResult> {
    const KEY_PREFIX: u32 = 0xFAFA0000;
    socket.0.write_u32(KEY_PREFIX).await?;
    socket.0.flush().await?;

    let encoded_public_key = state.key_pair.public.to_public_key_der()?;
    let mut challenge = vec![0; 16];
    rand::thread_rng().fill_bytes(&mut challenge);

    socket
        .0
        .write_u16(u32::from(encoded_public_key.len()) as u16)
        .await?;
    socket.0.write_all(encoded_public_key.as_bytes()).await?;
    socket.0.write_u16(challenge.len() as u16).await?;
    socket.0.write_all(&challenge).await?;
    socket.0.flush().await?;

    let mut encrypted_challenge = vec![0; socket.0.read_u16().await? as usize];
    socket.0.read_exact(&mut encrypted_challenge).await?;

    let mut encrypted_secret_key = vec![0; socket.0.read_u16().await? as usize];
    socket.0.read_exact(&mut encrypted_secret_key).await?;

    let secret_key =
        minecraft_crypt::decrypt_using_key(&state.key_pair.private, encrypted_secret_key)?;
    let auth_key = BigInt::from_signed_bytes_be(&minecraft_crypt::digest_data(
        "",
        &state.key_pair.public,
        &secret_key,
    )?)
    .to_str_radix(16);

    let requested_uuid = socket.0.read_uuid().await?;
    let requested_username = socket.0.read_string().await?;
    let connection_id = ConnectionId::new(socket.0.read_u64().await?)?;

    struct CipherPair {
        encrypt: Option<Aes128Cfb>,
        decrypt: Option<Aes128Cfb>,
    }
    let ciphers = if supports_encryption {
        CipherPair {
            encrypt: Some(minecraft_crypt::get_cipher(&secret_key)?),
            decrypt: Some(minecraft_crypt::get_cipher(&secret_key)?),
        }
    } else {
        CipherPair {
            encrypt: None,
            decrypt: None,
        }
    };

    if challenge
        != minecraft_crypt::decrypt_using_key(&state.key_pair.private, encrypted_challenge)?
    {
        return Ok(HandshakeResult {
            user_id: requested_uuid,
            connection_id,
            encrypt_cipher: ciphers.encrypt,
            decrypt_cipher: ciphers.decrypt,
            success: false,
            message: Some("Challenge failed".to_string()),
        });
    }

    let verify_result = verify_profile(
        state.session_service.as_ref(),
        requested_uuid,
        requested_username,
        auth_key,
    )
    .await;
    Ok(HandshakeResult {
        user_id: requested_uuid,
        connection_id,
        encrypt_cipher: ciphers.encrypt,
        decrypt_cipher: ciphers.decrypt,
        success: !verify_result.is_mismatch() || !verify_result.mismatch_is_error,
        message: if verify_result.is_mismatch() {
            Some(verify_result.message_with_uuid_info())
        } else {
            None
        },
    })
}

#[derive(Clone, Debug)]
struct VerifyProfileResult {
    requested_uuid: Uuid,
    expected_uuid: Uuid,
    mismatch_message: &'static str,
    mismatch_is_error: bool,
    include_uuid_info: bool,
}

impl VerifyProfileResult {
    fn is_mismatch(&self) -> bool {
        self.requested_uuid != self.expected_uuid
    }

    fn message_with_uuid_info(&self) -> String {
        if self.include_uuid_info {
            format!(
                "{} Client gave UUID {}. Expected UUID {}.",
                self.mismatch_message, self.requested_uuid, self.expected_uuid
            )
        } else {
            self.mismatch_message.to_string()
        }
    }
}

async fn verify_profile(
    session_service: &YggdrasilMinecraftSessionService,
    requested_uuid: Uuid,
    requested_username: String,
    auth_key: String,
) -> VerifyProfileResult {
    if requested_uuid.get_version_num() == 4 {
        let profile = session_service.has_joined_server(&requested_username, &auth_key)
            .await
            .unwrap_or_else(|_| {
                warn!("Authentication servers are down. Unable to verify {requested_username}. Will allow anyway.");
                Some(requested_uuid)
            });
        match profile {
            Some(uuid) => VerifyProfileResult {
                requested_uuid,
                expected_uuid: uuid,
                mismatch_message: "Mismatched UUID.",
                mismatch_is_error: true,
                include_uuid_info: true,
            },
            None => VerifyProfileResult {
                requested_uuid,
                expected_uuid: Uuid::nil(),
                mismatch_message: concat!(
                    "Failed to verify username. ",
                    "Please restart your game and the launcher. ",
                    "If you're unable to join regular Minecraft servers, this is not a bug with World Host.",
                ),
                mismatch_is_error: true,
                include_uuid_info: false,
            },
        }
    } else {
        let offline_uuid =
            java_name_uuid_from_bytes(format!("OfflinePlayer:{requested_username}").as_bytes());
        if requested_uuid.is_nil() || requested_uuid.is_max() {
            VerifyProfileResult {
                requested_uuid,
                expected_uuid: offline_uuid,
                mismatch_message: "Reserved special UUID not allowed.",
                mismatch_is_error: true,
                include_uuid_info: true,
            }
        } else {
            VerifyProfileResult {
                requested_uuid,
                expected_uuid: offline_uuid,
                mismatch_message:
                    "Mismatched offline UUID. Some features may not work as intended.",
                mismatch_is_error: false,
                include_uuid_info: true,
            }
        }
    }
}
