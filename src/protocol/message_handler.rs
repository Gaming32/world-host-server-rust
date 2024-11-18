use crate::connection::Connection;
use crate::protocol::c2s_message::WorldHostC2SMessage;
use crate::protocol::s2c_message::WorldHostS2CMessage;
use crate::protocol::security::SecurityLevel;
use crate::server_state::ServerState;
use crate::util::{add_with_circle_limit, remove_double_key};
use linked_hash_set::LinkedHashSet;
use log::warn;
use std::ops::DerefMut;
use uuid::Uuid;

pub async fn handle_message(
    message: WorldHostC2SMessage,
    connection: &Connection,
    server: &ServerState,
) -> anyhow::Result<()> {
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
            let other_connections = server.connections.lock().await.by_user_id(to_user);
            if !other_connections.is_empty() {
                for other in other_connections {
                    if other.id != connection.id {
                        send_safely(connection, &other, &response).await;
                    }
                }
            } else if connection.security_level() > SecurityLevel::Insecure {
                let removed_remembered = {
                    let mut remembered = server.remembered_friend_requests.lock().await;
                    let my_requests = remembered
                        .entry(connection.user_uuid)
                        .or_insert_with(LinkedHashSet::new);
                    add_with_circle_limit(my_requests, to_user, 5)
                };
                let removed_received = {
                    let mut received = server.received_friend_requests.lock().await;
                    if let Some(removed_remembered) = removed_remembered {
                        remove_double_key(
                            received.deref_mut(),
                            &removed_remembered,
                            &connection.user_uuid,
                        );
                    }
                    let my_remembered = received.entry(to_user).or_insert_with(LinkedHashSet::new);
                    add_with_circle_limit(my_remembered, connection.user_uuid, 10)
                };
                if let Some(removed_received) = removed_received {
                    remove_double_key(
                        server.remembered_friend_requests.lock().await.deref_mut(),
                        &removed_received,
                        &to_user,
                    );
                }
            }
        }
        PublishedWorld { friends } => {
            // TODO: Track online list
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
            // TODO: Track online list
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
                return Ok(());
            }
            let online = server.connections.lock().await.by_user_id(friend);
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
                return Ok(());
            }
            if connection_id != connection.id {
                if let Some(other) = server.connections.lock().await.by_id(connection_id) {
                    send_safely(connection, other, &response.unwrap()).await;
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
            return Box::pin(handle_message(
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
            // TODO: Proxy
        }
        ProxyDisconnect { connection_id } => {
            // TODO: Proxy
        }
        RequestDirectJoin { connection_id } => {
            if connection_id != connection.id {
                if let Some(other) = server.connections.lock().await.by_id(connection_id) {
                    send_safely(
                        connection,
                        other,
                        &WorldHostS2CMessage::RequestJoin {
                            user: connection.user_uuid,
                            connection_id: connection.id,
                            security: connection.security_level(),
                        },
                    )
                    .await;
                    return Ok(());
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
                return Ok(());
            }
            if let Some(other) = server.connections.lock().await.by_id(connection_id) {
                send_safely(
                    connection,
                    other,
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
            my_local_host,
            my_local_port,
        } => {
            if let Some(target_client) = server.connections.lock().await.by_id(target_connection) {
                if target_client.protocol_version < 7 {
                    send_safely(
                        connection,
                        connection,
                        &WorldHostS2CMessage::PunchRequestCancelled { punch_id },
                    )
                    .await;
                    return Ok(());
                }
                send_safely(
                    connection,
                    target_client,
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
            if let Some(target) = server.connections.lock().await.by_id(target_connection) {
                send_safely(
                    connection,
                    target,
                    &WorldHostS2CMessage::PunchRequestCancelled { punch_id },
                )
                .await;
            }
        }
        BeginPortLookup { lookup_id } => {
            // TODO: Port lookups
        }
        PunchSuccess {
            connection_id,
            punch_id,
            host,
            port,
        } => {
            if let Some(target) = server.connections.lock().await.by_id(connection_id) {
                send_safely(
                    connection,
                    target,
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
    Ok(())
}

async fn broadcast_to_friends(
    connection: &Connection,
    server: &ServerState,
    friends: Vec<Uuid>,
    message: WorldHostS2CMessage,
) {
    for friend in friends {
        for other in server.connections.lock().await.by_user_id(friend) {
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
