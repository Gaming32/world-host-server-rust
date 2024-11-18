use crate::connection::connection_id::ConnectionId;
use crate::connection::Connection;
use crate::json_data::ExternalProxy;
use crate::protocol::s2c_message::WorldHostS2CMessage;
use crate::server_state::{FullServerConfig, ServerState};
use crate::util::mc_packet::{MinecraftPacketAsyncRead, MinecraftPacketWrite};
use log::{error, info};
use std::process::exit;
use std::sync::Arc;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

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
            handle_proxy_connection(proxy_socket, connection_id, server.as_ref()).await;
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

async fn handle_proxy_connection(mut socket: TcpStream, connection_id: u64, server: &ServerState) {
    let mut connection = None;
    // Any error returned simply means the connection was closed, and we don't care.
    if let Err(error) = handle_inner(&mut socket, connection_id, server, &mut connection).await {
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
    socket: &mut TcpStream,
    connection_id: u64,
    server: &ServerState,
    connection_out: &mut Option<Connection>,
) -> io::Result<()> {
    let dest_cid = handshake(socket, &server.config).await?;
    if dest_cid.is_none() {
        return Ok(());
    }
    let dest_cid = dest_cid.unwrap();

    Ok(())
}

async fn handshake(
    socket: &mut TcpStream,
    config: &FullServerConfig,
) -> io::Result<Option<ConnectionId>> {
    socket.read_var_int().await?; // Packet size
    socket.read_var_int().await?; // Packet ID
    socket.read_var_int().await?; // Protocol version
    let this_addr = socket.read_mc_string(255).await?;
    let this_port = socket.read_u16().await?;
    let next_state = socket.read_var_int().await? as u8;

    let cid_str = &this_addr[..this_addr.find('.').unwrap_or(this_addr.len())];
    Ok(match cid_str.parse() {
        Ok(cid) => Some(cid),
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
        packet_data.write_string(format!(r#"{{"description":{json_message}}}"#), 32767)?;
    } else if next_state == 2 {
        packet_data.write_string(json_message, 262144)?;
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
