use crate::connection::connection_id::ConnectionId;
use crate::connection::Connection;
use crate::json_data::ExternalProxy;
use crate::protocol::s2c_message::WorldHostS2CMessage;
use crate::server_state::{FullServerConfig, ServerState};
use crate::util::mc_packet::{MinecraftPacketAsyncRead, MinecraftPacketRead, MinecraftPacketWrite};
use log::{debug, error, info};
use std::io::Cursor;
use std::net::IpAddr;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::time::{sleep, Instant};
use tokio_util::bytes::Buf;

pub async fn run_proxy_server(server: Arc<ServerState>) {
    if server.config.base_addr.is_none() {
        info!("Proxy server disabled by request");
        return;
    }
    if let Some(servers) = &server.config.external_servers {
        check_for_fallback_message(servers);
    }
    info!(
        "Starting proxy server on port {}",
        server.config.in_java_port
    );

    let listener = TcpListener::bind(("0.0.0.0", server.config.in_java_port))
        .await
        .unwrap_or_else(|error| {
            error!("Failed to start proxy server: {error}");
            exit(1);
        });

    let mut next_connection_id = 0u64;
    info!("Started proxy server on {}", listener.local_addr().unwrap());
    loop {
        let result = listener.accept().await;
        if let Err(error) = result {
            error!("Failed to accept proxy connection: {error}");
            continue;
        }
        let (proxy_socket, addr) = result.unwrap();

        let connection_id = next_connection_id;
        next_connection_id = next_connection_id.wrapping_add(1);
        info!("Accepted proxy connection {connection_id} from {addr}");

        let server = server.clone();
        tokio::spawn(async move {
            handle_proxy_connection(proxy_socket, addr.ip(), connection_id, server.as_ref()).await;
        });
    }
}

fn check_for_fallback_message(servers: &[Arc<ExternalProxy>]) {
    if servers.iter().any(|p| p.addr.is_none()) {
        return;
    }
    info!("Same-process proxy server is enabled, but it is not present in external_proxies.json. This means");
    info!("that it will be used only as a fallback if the client's best choice for external proxy goes down.");
}

async fn handle_proxy_connection(
    socket: TcpStream,
    remote_addr: IpAddr,
    connection_id: u64,
    server: &ServerState,
) {
    let mut connection = None;
    // Any error returned simply means the connection was closed, and we don't care.
    if let Err(error) =
        handle_inner(socket, remote_addr, connection_id, server, &mut connection).await
    {
        info!("Closing proxy connection {connection_id} due to {error}");
    }
    server.proxy_connections.lock().await.remove(&connection_id);
    if let Some(connection) = connection {
        // Same as above
        let _ = connection
            .send_message(&WorldHostS2CMessage::ProxyDisconnect { connection_id })
            .await;
    }
    info!("Proxy connection {connection_id} closed");
}

async fn handle_inner(
    mut socket: TcpStream,
    remote_addr: IpAddr,
    connection_id: u64,
    server: &ServerState,
    connection_out: &mut Option<Connection>,
) -> io::Result<()> {
    let handshake_result = handshake(&mut socket, &server.config).await?;
    if handshake_result.is_none() {
        return Ok(());
    }
    let HandshakeResult {
        connection_id: dest_cid,
        next_state,
        handshake_data,
    } = handshake_result.unwrap();

    let mut connection = {
        let connections = server.connections.lock().await;
        let connection = connections.by_id(dest_cid);
        if connection.is_none() {
            return disconnect(
                &mut socket,
                next_state,
                format!("Couldn't find server with ID {dest_cid}"),
            )
            .await;
        }
        connection.unwrap().clone()
    };
    *connection_out = Some(connection.clone());

    let (mut read, write) = socket.into_split();
    server
        .proxy_connections
        .lock()
        .await
        .insert(connection_id, (dest_cid, Mutex::new(write)));

    connection
        .send_message(&WorldHostS2CMessage::ProxyConnect {
            connection_id,
            remote_addr,
        })
        .await?;
    connection
        .send_message(&WorldHostS2CMessage::ProxyC2SPacket {
            connection_id,
            data: {
                let mut data = Vec::with_capacity(handshake_data.len() + 2);
                data.write_var_int(handshake_data.len() as i32)?;
                data.extend_from_slice(&handshake_data);
                drop(handshake_data);
                data
            },
        })
        .await?;

    let mut buffer = vec![0; 64 * 1024];
    loop {
        let n = read.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        let send_start = Instant::now();
        let failed = loop {
            let result = connection
                .send_message(&WorldHostS2CMessage::ProxyC2SPacket {
                    connection_id,
                    data: buffer[..n].to_vec(),
                })
                .await;
            if result.is_ok() {
                break false;
            }
            drop(result);
            let failed = loop {
                sleep(Duration::from_millis(50)).await;
                if let Some(new_connection) =
                    server.connections.lock().await.by_id(dest_cid).cloned()
                {
                    *connection_out = Some(new_connection.clone());
                    connection = new_connection;
                    break false;
                }
                if send_start.elapsed() > Duration::from_secs(5) {
                    break true;
                }
            };
            if failed {
                break true;
            }
        };
        if failed {
            break;
        }
    }

    Ok(())
}

struct HandshakeResult {
    connection_id: ConnectionId,
    next_state: u8,
    handshake_data: Vec<u8>,
}

async fn handshake(
    socket: &mut TcpStream,
    config: &FullServerConfig,
) -> io::Result<Option<HandshakeResult>> {
    let packet_size = socket.read_var_int().await? as usize;
    let mut handshake_data = vec![0; packet_size];
    socket.read_exact(&mut handshake_data).await?;

    let mut handshake_cursor = Cursor::new(handshake_data.as_slice());
    handshake_cursor.get_var_int()?; // Packet ID
    handshake_cursor.get_var_int()?; // Protocol version
    let this_addr = handshake_cursor.get_mc_string(255)?;
    let this_port = handshake_cursor.get_u16();
    let next_state = handshake_cursor.get_var_int()? as u8;

    let cid_str = &this_addr[..this_addr.find('.').unwrap_or(this_addr.len())];
    Ok(match cid_str.parse() {
        Ok(connection_id) => Some(HandshakeResult {
            connection_id,
            next_state,
            handshake_data,
        }),
        Err(error) => {
            disconnect(
                socket,
                next_state,
                if Some(&this_addr) == config.base_addr.as_ref() {
                    let show_addr = if this_port == 25565 {
                        this_addr
                    } else {
                        format!("{this_addr}:{this_port}")
                    };
                    format!("Please use the syntax my-connection-id.{show_addr}")
                } else {
                    format!("Invalid connection ID: {error}")
                },
            )
            .await?;
            None
        }
    })
}

async fn disconnect(socket: &mut TcpStream, next_state: u8, message: String) -> io::Result<()> {
    let json_message = format!(r#"{{"text":"{message}","color":"red"}}"#);

    let mut packet_data = vec![0x00];
    if next_state == 1 {
        packet_data.write_mc_string(format!(r#"{{"description":{json_message}}}"#), 32767)?;
    } else if next_state == 2 {
        packet_data.write_mc_string(json_message, 262144)?;
    }
    let mut packet = Vec::new();
    packet.write_var_int(packet_data.len() as i32)?;
    packet.extend_from_slice(&packet_data);
    socket.write_all(&packet).await?;
    socket.flush().await?;

    if next_state == 1 {
        packet_data.clear();
        packet_data.push(0x01);
        packet_data.extend_from_slice(&[0; 8]);
        packet.clear();
        packet.write_var_int(packet_data.len() as i32)?;
        packet.extend_from_slice(&packet_data);
        socket.write_all(&packet).await?;
        socket.flush().await?;
    }

    socket.shutdown().await
}
