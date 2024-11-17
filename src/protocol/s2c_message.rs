use crate::connection::connection_id::ConnectionId;
use crate::protocol::security_level::SecurityLevel;
use crate::serial_fields;
use crate::serialization::fielded::FieldedSerializer;
use crate::serialization::serializable::PacketSerializable;
use std::net::IpAddr;
use uuid::Uuid;

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
            Error { .. } => 0,
            IsOnlineTo { .. } => 1,
            OnlineGame { .. } => 2,
            FriendRequest { .. } => 3,
            PublishedWorld { .. } => 4,
            ClosedWorld { .. } => 5,
            RequestJoin { .. } => 6,
            QueryRequest { .. } => 7,
            QueryResponse { .. } => 8,
            ProxyC2SPacket { .. } => 9,
            ProxyConnect { .. } => 10,
            ProxyDisconnect { .. } => 11,
            ConnectionInfo { .. } => 12,
            ExternalProxyServer { .. } => 13,
            OutdatedWorldHost { .. } => 14,
            ConnectionNotFound { .. } => 15,
            NewQueryResponse { .. } => 16,
            Warning { .. } => 17,
            PunchOpenRequest { .. } => 18,
            CancelPortLookup { .. } => 19,
            PortLookupSuccess { .. } => 20,
            PunchRequestCancelled { .. } => 21,
            PunchSuccess { .. } => 22,
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
    fn fields(&self) -> Vec<Box<&(dyn PacketSerializable + '_)>> {
        use WorldHostS2CMessage::*;
        match self {
            Error { message, critical } => serial_fields!(message, critical),
            IsOnlineTo { user } => serial_fields!(user),
            OnlineGame {
                host,
                port,
                owner_cid,
            } => serial_fields!(host, port, owner_cid, &false),
            FriendRequest {
                from_user,
                security,
            } => serial_fields!(from_user, security),
            PublishedWorld {
                user,
                connection_id,
                security,
            } => serial_fields!(user, connection_id, security),
            ClosedWorld { user } => serial_fields!(user),
            RequestJoin {
                user,
                connection_id,
                security,
            } => serial_fields!(user, connection_id, security),
            QueryRequest {
                friend,
                connection_id,
                security,
            } => serial_fields!(friend, connection_id, security),
            QueryResponse {
                friend,
                length,
                data,
            } => serial_fields!(friend, length, data),
            ProxyC2SPacket {
                connection_id,
                data,
            } => serial_fields!(connection_id, data),
            ProxyConnect {
                connection_id,
                remote_addr,
            } => serial_fields!(connection_id, remote_addr),
            ProxyDisconnect { connection_id } => serial_fields!(connection_id),
            ConnectionInfo {
                connection_id,
                base_ip,
                base_port,
                user_ip,
                protocol_version,
                punch_port,
            } => serial_fields!(
                connection_id,
                base_ip,
                base_port,
                user_ip,
                protocol_version,
                punch_port
            ),
            ExternalProxyServer {
                host,
                port,
                base_addr,
                mc_port,
            } => serial_fields!(host, port, base_addr, mc_port),
            OutdatedWorldHost {
                recommended_version,
            } => serial_fields!(recommended_version),
            ConnectionNotFound { connection_id } => serial_fields!(connection_id),
            NewQueryResponse { friend, data } => serial_fields!(friend, data),
            Warning { message, important } => serial_fields!(message, important),
            PunchOpenRequest {
                punch_id,
                purpose,
                from_host,
                from_port,
                connection_id,
                user,
                security,
            } => serial_fields!(
                punch_id,
                purpose,
                from_host,
                from_port,
                connection_id,
                user,
                security
            ),
            CancelPortLookup { lookup_id } => serial_fields!(lookup_id),
            PortLookupSuccess {
                lookup_id,
                host,
                port,
            } => serial_fields!(lookup_id, host, port),
            PunchRequestCancelled { punch_id } => serial_fields!(punch_id),
            PunchSuccess {
                punch_id,
                host,
                port,
            } => serial_fields!(punch_id, host, port),
        }
    }
}
