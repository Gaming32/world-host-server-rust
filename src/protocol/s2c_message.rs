use crate::connection::connection_id::ConnectionId;
use crate::protocol::security::SecurityLevel;
use crate::serialization::fielded::FieldedSerializer;
use crate::serialization::serializable::PacketSerializable;
use std::net::IpAddr;
use uuid::Uuid;

pub const ERROR_ID: u8 = 0;
pub const IS_ONLINE_TO_ID: u8 = 1;
pub const ONLINE_GAME_ID: u8 = 2;
pub const FRIEND_REQUEST_ID: u8 = 3;
pub const PUBLISHED_WORLD_ID: u8 = 4;
pub const CLOSED_WORLD_ID: u8 = 5;
pub const REQUEST_JOIN_ID: u8 = 6;
pub const QUERY_REQUEST_ID: u8 = 7;
pub const QUERY_RESPONSE_ID: u8 = 8;
pub const PROXY_C2S_PACKET_ID: u8 = 9;
pub const PROXY_CONNECT_ID: u8 = 10;
pub const PROXY_DISCONNECT_ID: u8 = 11;
pub const CONNECTION_INFO_ID: u8 = 12;
pub const EXTERNAL_PROXY_SERVER_ID: u8 = 13;
pub const OUTDATED_WORLD_HOST_ID: u8 = 14;
pub const CONNECTION_NOT_FOUND_ID: u8 = 15;
pub const NEW_QUERY_RESPONSE_ID: u8 = 16;
pub const WARNING_ID: u8 = 17;
pub const PUNCH_OPEN_REQUEST_ID: u8 = 18;
pub const CANCEL_PORT_LOOKUP_ID: u8 = 19;
pub const PORT_LOOKUP_SUCCESS_ID: u8 = 20;
pub const PUNCH_REQUEST_CANCELLED_ID: u8 = 21;
pub const PUNCH_SUCCESS_ID: u8 = 22;

#[derive(Clone, Debug)]
pub enum WorldHostS2CMessage {
    Error {
        message: String,
        critical: bool,
    },
    IsOnlineTo {
        user: Uuid,
    },
    OnlineGame {
        host: String,
        port: u16,
        owner_cid: ConnectionId,
    },
    FriendRequest {
        from_user: Uuid,
        security: SecurityLevel,
    },
    PublishedWorld {
        user: Uuid,
        connection_id: ConnectionId,
        security: SecurityLevel,
    },
    ClosedWorld {
        user: Uuid,
    },
    RequestJoin {
        user: Uuid,
        connection_id: ConnectionId,
        security: SecurityLevel,
    },
    QueryRequest {
        friend: Uuid,
        connection_id: ConnectionId,
        security: SecurityLevel,
    },
    #[deprecated = "QueryResponse uses an old format. NewQueryResponse should be used instead."]
    QueryResponse {
        friend: Uuid,
        length: u32,
        data: Vec<u8>,
    },
    ProxyC2SPacket {
        connection_id: u64,
        data: Vec<u8>,
    },
    ProxyConnect {
        connection_id: u64,
        remote_addr: IpAddr,
    },
    ProxyDisconnect {
        connection_id: u64,
    },
    ConnectionInfo {
        connection_id: ConnectionId,
        base_ip: String,
        base_port: u16,
        user_ip: String,
        protocol_version: u32,
        punch_port: u16,
    },
    ExternalProxyServer {
        host: String,
        port: u16,
        base_addr: String,
        mc_port: u16,
    },
    OutdatedWorldHost {
        recommended_version: String,
    },
    ConnectionNotFound {
        connection_id: ConnectionId,
    },
    NewQueryResponse {
        friend: Uuid,
        data: Vec<u8>,
    },
    Warning {
        message: String,
        important: bool,
    },
    PunchOpenRequest {
        punch_id: Uuid,
        purpose: String,
        from_host: String,
        from_port: u16,
        connection_id: ConnectionId,
        user: Uuid,
        security: SecurityLevel,
    },
    CancelPortLookup {
        lookup_id: Uuid,
    },
    PortLookupSuccess {
        lookup_id: Uuid,
        host: String,
        port: u16,
    },
    PunchRequestCancelled {
        punch_id: Uuid,
    },
    PunchSuccess {
        punch_id: Uuid,
        host: String,
        port: u16,
    },
}

impl WorldHostS2CMessage {
    #[allow(deprecated)]
    pub fn type_id(&self) -> u8 {
        use WorldHostS2CMessage::*;
        match self {
            Error { .. } => ERROR_ID,
            IsOnlineTo { .. } => IS_ONLINE_TO_ID,
            OnlineGame { .. } => ONLINE_GAME_ID,
            FriendRequest { .. } => FRIEND_REQUEST_ID,
            PublishedWorld { .. } => PUBLISHED_WORLD_ID,
            ClosedWorld { .. } => CLOSED_WORLD_ID,
            RequestJoin { .. } => REQUEST_JOIN_ID,
            QueryRequest { .. } => QUERY_REQUEST_ID,
            QueryResponse { .. } => QUERY_RESPONSE_ID,
            ProxyC2SPacket { .. } => PROXY_C2S_PACKET_ID,
            ProxyConnect { .. } => PROXY_CONNECT_ID,
            ProxyDisconnect { .. } => PROXY_DISCONNECT_ID,
            ConnectionInfo { .. } => CONNECTION_INFO_ID,
            ExternalProxyServer { .. } => EXTERNAL_PROXY_SERVER_ID,
            OutdatedWorldHost { .. } => OUTDATED_WORLD_HOST_ID,
            ConnectionNotFound { .. } => CONNECTION_NOT_FOUND_ID,
            NewQueryResponse { .. } => NEW_QUERY_RESPONSE_ID,
            Warning { .. } => WARNING_ID,
            PunchOpenRequest { .. } => PUNCH_OPEN_REQUEST_ID,
            CancelPortLookup { .. } => CANCEL_PORT_LOOKUP_ID,
            PortLookupSuccess { .. } => PORT_LOOKUP_SUCCESS_ID,
            PunchRequestCancelled { .. } => PUNCH_REQUEST_CANCELLED_ID,
            PunchSuccess { .. } => PUNCH_SUCCESS_ID,
        }
    }

    #[allow(deprecated)]
    pub fn first_protocol(&self) -> u32 {
        use WorldHostS2CMessage::*;
        match self {
            Error { .. } => 2,
            IsOnlineTo { .. } => 2,
            OnlineGame { .. } => 2,
            FriendRequest { .. } => 2,
            PublishedWorld { .. } => 2,
            ClosedWorld { .. } => 2,
            RequestJoin { .. } => 2,
            QueryRequest { .. } => 2,
            QueryResponse { .. } => 2,
            ProxyC2SPacket { .. } => 2,
            ProxyConnect { .. } => 2,
            ProxyDisconnect { .. } => 2,
            ConnectionInfo { .. } => 2,
            ExternalProxyServer { .. } => 2,
            OutdatedWorldHost { .. } => 4,
            ConnectionNotFound { .. } => 4,
            NewQueryResponse { .. } => 5,
            Warning { .. } => 6,
            PunchOpenRequest { .. } => 7,
            CancelPortLookup { .. } => 7,
            PortLookupSuccess { .. } => 7,
            PunchRequestCancelled { .. } => 7,
            PunchSuccess { .. } => 7,
        }
    }
}

impl FieldedSerializer for WorldHostS2CMessage {
    #[allow(deprecated)]
    fn fields(&self) -> Vec<&(dyn PacketSerializable + '_)> {
        use WorldHostS2CMessage::*;
        match self {
            Error { message, critical } => vec![message, critical],
            IsOnlineTo { user } => vec![user],
            OnlineGame {
                host,
                port,
                owner_cid,
            } => vec![host, port, owner_cid, &false],
            FriendRequest {
                from_user,
                security,
            } => vec![from_user, security],
            PublishedWorld {
                user,
                connection_id,
                security,
            } => vec![user, connection_id, security],
            ClosedWorld { user } => vec![user],
            RequestJoin {
                user,
                connection_id,
                security,
            } => vec![user, connection_id, security],
            QueryRequest {
                friend,
                connection_id,
                security,
            } => vec![friend, connection_id, security],
            QueryResponse {
                friend,
                length,
                data,
            } => vec![friend, length, data],
            ProxyC2SPacket {
                connection_id,
                data,
            } => vec![connection_id, data],
            ProxyConnect {
                connection_id,
                remote_addr,
            } => vec![connection_id, remote_addr],
            ProxyDisconnect { connection_id } => vec![connection_id],
            ConnectionInfo {
                connection_id,
                base_ip,
                base_port,
                user_ip,
                protocol_version,
                punch_port,
            } => vec![
                connection_id,
                base_ip,
                base_port,
                user_ip,
                protocol_version,
                punch_port,
            ],
            ExternalProxyServer {
                host,
                port,
                base_addr,
                mc_port,
            } => vec![host, port, base_addr, mc_port],
            OutdatedWorldHost {
                recommended_version,
            } => vec![recommended_version],
            ConnectionNotFound { connection_id } => vec![connection_id],
            NewQueryResponse { friend, data } => vec![friend, data],
            Warning { message, important } => vec![message, important],
            PunchOpenRequest {
                punch_id,
                purpose,
                from_host,
                from_port,
                connection_id,
                user,
                security,
            } => vec![
                punch_id,
                purpose,
                from_host,
                from_port,
                connection_id,
                user,
                security,
            ],
            CancelPortLookup { lookup_id } => vec![lookup_id],
            PortLookupSuccess {
                lookup_id,
                host,
                port,
            } => vec![lookup_id, host, port],
            PunchRequestCancelled { punch_id } => vec![punch_id],
            PunchSuccess {
                punch_id,
                host,
                port,
            } => vec![punch_id, host, port],
        }
    }
}
