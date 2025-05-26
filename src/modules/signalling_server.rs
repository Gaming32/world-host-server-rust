use crate::protocol::s2c_message::WorldHostS2CMessage;
use crate::server_state::ServerState;
use crate::util::copy_to_fixed_size;
use log::{error, info, warn};
use queues::IsQueue;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::{Instant, MissedTickBehavior, interval_at};
use uuid::Uuid;

pub async fn run_signalling_server(server: Arc<ServerState>) {
    info!("Starting signalling server on port {}", server.config.port);

    let listener = UdpSocket::bind(("0.0.0.0", server.config.port))
        .await
        .unwrap_or_else(|error| {
            error!("Failed to start signalling server: {error}");
            exit(1);
        });
    info!(
        "Started signalling server on {}",
        listener.local_addr().unwrap()
    );

    {
        let server = server.clone();
        tokio::spawn(async move {
            const PUMP_TIME: Duration = Duration::from_secs(1);
            let mut interval = interval_at(Instant::now() + PUMP_TIME, PUMP_TIME);
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
            loop {
                interval.tick().await;
                cleanup_expired_punch_requests(server.as_ref()).await;
            }
        });
    }

    let mut signal = vec![0; 16];
    loop {
        let result = listener.recv_from(&mut signal).await;
        if let Err(error) = result {
            error!("Failed to receive signal: {error}");
            continue;
        }
        let (read, addr) = result.unwrap();
        if read < 16 {
            warn!("Received invalid signal from {addr}: {read} bytes is fewer than 16");
            continue;
        }

        let signal = copy_to_fixed_size(&signal);
        let server = server.clone();
        tokio::spawn(async move {
            let lookup_id = Uuid::from_bytes(signal);
            if let Some((_, request)) = server.port_lookups.remove(&lookup_id) {
                if let Some(connection) = server.connections.by_id(request.source_client) {
                    // If it's already been closed, well there's nothing we can do about it
                    let _ = connection
                        .send_message(&WorldHostS2CMessage::PortLookupSuccess {
                            lookup_id,
                            host: addr.ip().to_string(),
                            port: addr.port(),
                        })
                        .await;
                }
            }
        });
    }
}

async fn cleanup_expired_punch_requests(server: &ServerState) {
    let time = Instant::now();
    let mut lookups = server.port_lookup_by_expiry.lock().await;
    while let Ok((expiry, request)) = lookups.peek() {
        if time > expiry {
            lookups.remove().unwrap();
            if server.port_lookups.remove(&request.lookup_id).is_none() {
                continue;
            }
            if let Some(connection) = server.connections.by_id(request.source_client) {
                let _ = connection
                    .send_message(&WorldHostS2CMessage::CancelPortLookup {
                        lookup_id: request.lookup_id,
                    })
                    .await;
            }
        } else {
            break;
        }
    }
}
