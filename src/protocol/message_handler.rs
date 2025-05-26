use crate::connection::Connection;
use crate::protocol::c2s_message::WorldHostC2SMessage;
use crate::protocol::port_lookup::{ActivePortLookup, PORT_LOOKUP_EXPIRY};
use crate::protocol::s2c_message::WorldHostS2CMessage;
use crate::protocol::security::SecurityLevel;
use crate::server_state::ServerState;
use crate::util::{add_with_circle_limit, remove_double_key};
use log::warn;
use queues::IsQueue;
use tokio::io::AsyncWriteExt;
use tokio::time::Instant;
use uuid::Uuid;

pub async fn handle_message(
    message: WorldHostC2SMessage,
    connection: &Connection,
    server: &ServerState,
) {
    use WorldHostC2SMessage::*;
    match message {
        ListOnline { friends } => {
            broadcast_to_friends(
                connection,
                server,
                friends,
                WorldHostS2CMessage::IsOnlineTo {
                    user: connection.user_uuid,
                },
            )
            .await;
        }
        FriendRequest { to_user } => {
            let response = WorldHostS2CMessage::FriendRequest {
                from_user: connection.user_uuid,
                security: connection.security_level(),
            };
            let other_connections = server.connections.by_user_id(to_user);
            if !other_connections.is_empty() {
                for other in other_connections {
                    if other.id != connection.id {
                        send_safely(connection, &other, &response).await;
                    }
                }
            } else if connection.security_level() > SecurityLevel::Insecure {
                let removed_remembered = {
                    let mut my_requests = server
                        .remembered_friend_requests
                        .entry(connection.user_uuid)
                        .or_default();
                    add_with_circle_limit(&mut my_requests, to_user, 5)
                };
                let removed_received = {
                    if let Some(removed_remembered) = removed_remembered {
                        remove_double_key(
                            &server.received_friend_requests,
                            &removed_remembered,
                            &connection.user_uuid,
                        );
                    }
                    let mut my_remembered =
                        server.received_friend_requests.entry(to_user).or_default();
                    add_with_circle_limit(&mut my_remembered, connection.user_uuid, 10)
                };
                if let Some(removed_received) = removed_received {
                    remove_double_key(
                        &server.remembered_friend_requests,
                        &removed_received,
                        &to_user,
                    );
                }
            }
        }
        PublishedWorld { friends } => {
            connection
                .state
                .lock()
                .await
                .open_to_friends
                .extend(friends.iter());
            broadcast_to_friends(
                connection,
                server,
                friends,
                WorldHostS2CMessage::PublishedWorld {
                    user: connection.user_uuid,
                    connection_id: connection.id,
                    security: connection.security_level(),
                },
            )
            .await;
        }
        ClosedWorld { friends } => {
            {
                let open = &mut connection.state.lock().await.open_to_friends;
                for friend in friends.iter() {
                    open.remove(friend);
                }
            }
            broadcast_to_friends(
                connection,
                server,
                friends,
                WorldHostS2CMessage::ClosedWorld {
                    user: connection.user_uuid,
                },
            )
            .await;
        }
        RequestJoin { friend } => {
            if connection.protocol_version >= 4 {
                warn!(
                    "Connection {} tried to use unsupported RequestJoin message",
                    connection.id
                );
                send_safely(connection, connection, &WorldHostS2CMessage::Error {
                    message: "Please use the v4+ RequestDirectJoin message instead of the unsupported RequestJoin message".to_string(),
                    critical: false
                }).await;
                return;
            }
            let online = server.connections.by_user_id(friend);
            if !online.is_empty() {
                if let Some(last) = online.last() {
                    send_safely(
                        connection,
                        last,
                        &WorldHostS2CMessage::RequestJoin {
                            user: connection.user_uuid,
                            connection_id: connection.id,
                            security: connection.security_level(),
                        },
                    )
                    .await;
                }
            }
        }
        JoinGranted {
            connection_id,
            join_type,
        } => {
            let response = join_type.to_online_game(connection, &server.config).await;
            if response.is_none() {
                send_safely(
                    connection,
                    connection,
                    &WorldHostS2CMessage::Error {
                        message: format!("This server does not support JoinType {join_type:?}"),
                        critical: false,
                    },
                )
                .await;
                return;
            }
            if connection_id != connection.id {
                if let Some(other) = server.connections.by_id(connection_id) {
                    send_safely(connection, &other, &response.unwrap()).await;
                }
            }
        }
        QueryRequest { friends } => {
            broadcast_to_friends(
                connection,
                server,
                friends,
                WorldHostS2CMessage::QueryRequest {
                    friend: connection.user_uuid,
                    connection_id: connection.id,
                    security: connection.security_level(),
                },
            )
            .await;
        }
        QueryResponse {
            connection_id,
            data,
        } => {
            Box::pin(handle_message(
                NewQueryResponse {
                    connection_id,
                    data,
                },
                connection,
                server,
            ))
            .await;
        }
        ProxyS2CPacket {
            connection_id,
            data,
        } => {
            if let Some(proxy_connection) = server.proxy_connections.get(&connection_id) {
                let (cid, socket) = proxy_connection.value();
                if *cid == connection.id {
                    let mut socket = socket.lock().await;
                    // Socket may be disconnected. Let the receiver deal with that.
                    let _ = socket.write_all(&data).await;
                    let _ = socket.flush().await;
                }
            }
        }
        ProxyDisconnect { connection_id } => {
            if let Some(proxy_connection) = server.proxy_connections.get(&connection_id) {
                let (cid, socket) = proxy_connection.value();
                if *cid == connection.id {
                    // Socket may already be shutdown. That's the receiver's job to handle.
                    let _ = socket.lock().await.shutdown().await;
                }
            }
        }
        RequestDirectJoin { connection_id } => {
            if connection_id != connection.id {
                if let Some(other) = server.connections.by_id(connection_id) {
                    send_safely(
                        connection,
                        &other,
                        &WorldHostS2CMessage::RequestJoin {
                            user: connection.user_uuid,
                            connection_id: connection.id,
                            security: connection.security_level(),
                        },
                    )
                    .await;
                    return;
                }
            }
            send_safely(
                connection,
                connection,
                &WorldHostS2CMessage::ConnectionNotFound { connection_id },
            )
            .await;
        }
        NewQueryResponse {
            connection_id,
            data,
        } => {
            if connection_id == connection.id {
                return;
            }
            if let Some(other) = server.connections.by_id(connection_id) {
                send_safely(
                    connection,
                    &other,
                    &if other.protocol_version < 5 {
                        #[allow(deprecated)]
                        WorldHostS2CMessage::QueryResponse {
                            friend: connection.user_uuid,
                            length: data.len() as u32,
                            data,
                        }
                    } else {
                        WorldHostS2CMessage::NewQueryResponse {
                            friend: connection.user_uuid,
                            data,
                        }
                    },
                )
                .await;
            }
        }
        RequestPunchOpen {
            target_connection,
            purpose,
            punch_id,
            my_host,
            my_port,
            my_local_host: _,
            my_local_port: _,
        } => {
            if let Some(target_client) = server.connections.by_id(target_connection) {
                if target_client.protocol_version < 7 {
                    send_safely(
                        connection,
                        connection,
                        &WorldHostS2CMessage::PunchRequestCancelled { punch_id },
                    )
                    .await;
                    return;
                }
                send_safely(
                    connection,
                    &target_client,
                    &WorldHostS2CMessage::PunchOpenRequest {
                        punch_id,
                        purpose,
                        from_host: my_host,
                        from_port: my_port,
                        connection_id: connection.id,
                        user: connection.user_uuid,
                        security: connection.security_level(),
                    },
                )
                .await;
            } else {
                send_safely(
                    connection,
                    connection,
                    &WorldHostS2CMessage::PunchRequestCancelled { punch_id },
                )
                .await;
            }
        }
        PunchFailed {
            target_connection,
            punch_id,
        } => {
            if let Some(target) = server.connections.by_id(target_connection) {
                send_safely(
                    connection,
                    &target,
                    &WorldHostS2CMessage::PunchRequestCancelled { punch_id },
                )
                .await;
            }
        }
        BeginPortLookup { lookup_id } => {
            let request = ActivePortLookup {
                lookup_id,
                source_client: connection.id,
            };
            server.port_lookups.insert(lookup_id, request);
            server
                .port_lookup_by_expiry
                .lock()
                .await
                .add((Instant::now() + PORT_LOOKUP_EXPIRY, request))
                .unwrap();
        }
        PunchSuccess {
            connection_id,
            punch_id,
            host,
            port,
        } => {
            if let Some(target) = server.connections.by_id(connection_id) {
                send_safely(
                    connection,
                    &target,
                    &WorldHostS2CMessage::PunchSuccess {
                        punch_id,
                        host,
                        port,
                    },
                )
                .await;
            }
        }
    }
}

async fn broadcast_to_friends(
    connection: &Connection,
    server: &ServerState,
    friends: Vec<Uuid>,
    message: WorldHostS2CMessage,
) {
    for friend in friends {
        for other in server.connections.by_user_id(friend) {
            if other.id != connection.id {
                send_safely(connection, &other, &message).await;
            }
        }
    }
}

async fn send_safely(from: &Connection, to: &Connection, message: &WorldHostS2CMessage) {
    if let Err(error) = to.send_message(message).await {
        warn!(
            "Failed to broadcast {message:?} from {} to {}: {error}",
            from.id, to.id
        );
    }
}
