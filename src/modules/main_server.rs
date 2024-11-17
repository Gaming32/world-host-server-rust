use crate::authlib::auth_service::YggdrasilAuthenticationService;
use crate::authlib::session_service::YggdrasilMinecraftSessionService;
use crate::connection::connection_id::ConnectionId;
use crate::connection::{Connection, LiveConnection};
use crate::minecraft_crypt;
use crate::minecraft_crypt::RsaKeyPair;
use crate::protocol::data_ext::WHAsyncReadExt;
use crate::protocol::protocol_versions;
use crate::protocol::s2c_message::WorldHostS2CMessage;
use crate::ratelimit::bucket::RateLimitBucket;
use crate::ratelimit::limiter::RateLimiter;
use crate::server_state::ServerState;
use crate::socket_wrapper::SocketWrapper;
use crate::util::ip_info_map::IpInfoMap;
use crate::util::java_util::java_name_uuid_from_bytes;
use anyhow::bail;
use log::{error, info, warn};
use num_bigint::BigInt;
use rand::RngCore;
use rsa::pkcs8::EncodePublicKey;
use std::net::IpAddr;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::time::{interval_at, Instant, MissedTickBehavior};
use uuid::Uuid;

pub async fn run_main_server(server: Arc<ServerState>) {
    let session_service = Arc::new(YggdrasilAuthenticationService::new().create_session_service());
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
        let session_service = session_service.clone();
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
            if let Err(error) = handle_connection(
                &session_service,
                &key_pair,
                socket,
                addr.ip(),
                &mut connection,
            )
            .await
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
    session_service: &YggdrasilMinecraftSessionService,
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

    let connection = match create_connection(
        socket,
        remote_addr,
        session_service,
        key_pair,
        protocol_version,
    )
    .await
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
    session_service: &YggdrasilMinecraftSessionService,
    key_pair: &RsaKeyPair,
    protocol_version: u32,
) -> Option<Connection> {
    let handshake_result =
        perform_versioned_handshake(&mut socket, session_service, key_pair, protocol_version).await;
    if let Err(error) = handshake_result {
        warn!("Failed to perform handshake from {remote_addr}: {error}");
        socket.send_close_error(error.to_string()).await;
        return None;
    }
    let handshake_result = handshake_result.unwrap();

    if handshake_result.success {
        if let Some(warning) = handshake_result.message {
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
    } else {
        let message = handshake_result.message.unwrap();
        warn!("Handshake from {remote_addr} failed: {message}");
        socket.send_close_error(message).await;
        return None;
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
    session_service: &YggdrasilMinecraftSessionService,
    key_pair: &RsaKeyPair,
    protocol_version: u32,
) -> anyhow::Result<HandshakeResult> {
    if protocol_version < protocol_versions::NEW_AUTH_PROTOCOL {
        Ok(HandshakeResult {
            user_id: socket.0.read_uuid().await?,
            connection_id: ConnectionId::new(socket.0.read_u64().await?)?,
            success: true,
            message: None,
        })
    } else {
        perform_handshake(
            socket,
            session_service,
            key_pair,
            protocol_version >= protocol_versions::ENCRYPTED_PROTOCOL,
        )
        .await
    }
}

#[derive(Clone, Debug)]
struct HandshakeResult {
    user_id: Uuid,
    connection_id: ConnectionId,
    // TODO: Encryption
    success: bool,
    message: Option<String>,
}

async fn perform_handshake(
    socket: &mut SocketWrapper,
    session_service: &YggdrasilMinecraftSessionService,
    key_pair: &RsaKeyPair,
    supports_encryption: bool,
) -> anyhow::Result<HandshakeResult> {
    const KEY_PREFIX: u32 = 0xFAFA0000;
    socket.0.write_u32(KEY_PREFIX).await?;
    socket.0.flush().await?;

    let encoded_public_key = key_pair.public.to_public_key_der()?;
    let mut challenge = vec![0u8; 16];
    rand::thread_rng().fill_bytes(&mut challenge);

    socket
        .0
        .write_u16(u32::from(encoded_public_key.len()) as u16)
        .await?;
    socket.0.write_all(encoded_public_key.as_bytes()).await?;
    socket.0.write_u16(challenge.len() as u16).await?;
    socket.0.write_all(&challenge).await?;
    socket.0.flush().await?;

    let mut encrypted_challenge = vec![0u8; socket.0.read_u16().await? as usize];
    socket.0.read_exact(&mut encrypted_challenge).await?;

    let mut encrypted_secret_key = vec![0u8; socket.0.read_u16().await? as usize];
    socket.0.read_exact(&mut encrypted_secret_key).await?;

    let secret_key = minecraft_crypt::decrypt_using_key(&key_pair.private, encrypted_secret_key)?;
    let auth_key = BigInt::from_signed_bytes_be(&minecraft_crypt::digest_data(
        "",
        &key_pair.public,
        &secret_key,
    )?)
    .to_str_radix(16);

    let requested_uuid = socket.0.read_uuid().await?;
    let requested_username = socket.0.read_string().await?;
    let connection_id = ConnectionId::new(socket.0.read_u64().await?)?;

    if challenge != minecraft_crypt::decrypt_using_key(&key_pair.private, encrypted_challenge)? {
        return Ok(HandshakeResult {
            user_id: requested_uuid,
            connection_id,
            // TODO: Encryption
            success: false,
            message: Some("Challenge failed".to_string()),
        });
    }

    let verify_result = verify_profile(
        session_service,
        requested_uuid,
        requested_username,
        auth_key,
    )
    .await;
    if verify_result.is_mismatch() && verify_result.mismatch_is_error {
        bail!(verify_result.message_with_uuid_info());
    }

    Ok(HandshakeResult {
        user_id: requested_uuid,
        connection_id,
        // TODO: Encryption,
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
