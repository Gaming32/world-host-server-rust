use crate::connection::connection_id::ConnectionId;
use crate::invalid_data;
use crate::protocol::data_ext::WHReadBytesExt;
use crate::protocol::join_type::JoinType;
use byteorder::{BigEndian, ReadBytesExt};
use std::io;
use std::io::{Cursor, Read};
use tokio_util::bytes::Buf;
use uuid::Uuid;

pub const LIST_ONLINE_ID: u8 = 0;
pub const FRIEND_REQUEST_ID: u8 = 1;
pub const PUBLISHED_WORLD_ID: u8 = 2;
pub const CLOSED_WORLD_ID: u8 = 3;
pub const REQUEST_JOIN_ID: u8 = 4;
pub const JOIN_GRANTED_ID: u8 = 5;
pub const QUERY_REQUEST_ID: u8 = 6;
pub const QUERY_RESPONSE_ID: u8 = 7;
pub const PROXY_S2C_PACKET_ID: u8 = 8;
pub const PROXY_DISCONNECT_ID: u8 = 9;
pub const REQUEST_DIRECT_JOIN_ID: u8 = 10;
pub const NEW_QUERY_RESPONSE_ID: u8 = 11;
pub const REQUEST_PUNCH_OPEN_ID: u8 = 12;
pub const PUNCH_FAILED_ID: u8 = 13;
pub const BEGIN_PORT_LOOKUP_ID: u8 = 14;
pub const PUNCH_SUCCESS_ID: u8 = 15;

#[derive(Clone, Debug)]
pub enum WorldHostC2SMessage {
    ListOnline {
        friends: Vec<Uuid>,
    },
    FriendRequest {
        to_user: Uuid,
    },
    PublishedWorld {
        friends: Vec<Uuid>,
    },
    ClosedWorld {
        friends: Vec<Uuid>,
    },
    RequestJoin {
        friend: Uuid,
    },
    JoinGranted {
        connection_id: ConnectionId,
        join_type: JoinType,
    },
    QueryRequest {
        friends: Vec<Uuid>,
    },
    QueryResponse {
        connection_id: ConnectionId,
        data: Vec<u8>,
    },
    ProxyS2CPacket {
        connection_id: u64,
        data: Vec<u8>,
    },
    ProxyDisconnect {
        connection_id: u64,
    },
    RequestDirectJoin {
        connection_id: ConnectionId,
    },
    NewQueryResponse {
        connection_id: ConnectionId,
        data: Vec<u8>,
    },
    RequestPunchOpen {
        target_connection: ConnectionId,
        purpose: String,
        punch_id: Uuid,
        my_host: String,
        my_port: u16,
        #[allow(dead_code)]
        my_local_host: String,
        #[allow(dead_code)]
        my_local_port: u16,
    },
    PunchFailed {
        target_connection: ConnectionId,
        punch_id: Uuid,
    },
    BeginPortLookup {
        lookup_id: Uuid,
    },
    PunchSuccess {
        connection_id: ConnectionId,
        punch_id: Uuid,
        host: String,
        port: u16,
    },
}

impl WorldHostC2SMessage {
    pub fn parse(id: u8, data: &[u8], max_protocol_version: Option<u32>) -> io::Result<Self> {
        let first_protocol = first_protocol_version(id);
        if first_protocol.is_none() {
            invalid_data!("Received message with unknown typeId from client: {id}");
        }
        let first_protocol = first_protocol.unwrap();
        if let Some(max_protocol) = max_protocol_version {
            if first_protocol > max_protocol {
                invalid_data!("Received too new message from client. Client has version {max_protocol}, but message ID {id} was added in {first_protocol}.");
            }
        }
        Self::parse_raw(id, &mut Cursor::new(data))
    }

    pub fn parse_raw(id: u8, cursor: &mut Cursor<&[u8]>) -> io::Result<Self> {
        use WorldHostC2SMessage::*;
        match id {
            LIST_ONLINE_ID => Ok(ListOnline {
                friends: Self::read_uuid_vec(cursor)?,
            }),
            FRIEND_REQUEST_ID => Ok(FriendRequest {
                to_user: cursor.read_uuid()?,
            }),
            PUBLISHED_WORLD_ID => Ok(PublishedWorld {
                friends: Self::read_uuid_vec(cursor)?,
            }),
            CLOSED_WORLD_ID => Ok(ClosedWorld {
                friends: Self::read_uuid_vec(cursor)?,
            }),
            REQUEST_JOIN_ID => Ok(RequestJoin {
                friend: cursor.read_uuid()?,
            }),
            JOIN_GRANTED_ID => Ok(JoinGranted {
                connection_id: cursor.read_connection_id()?,
                join_type: JoinType::decode(cursor)?,
            }),
            QUERY_REQUEST_ID => Ok(QueryRequest {
                friends: Self::read_uuid_vec(cursor)?,
            }),
            QUERY_RESPONSE_ID => {
                let connection_id = cursor.read_connection_id()?;
                let len = cursor.read_u32::<BigEndian>()? as usize;
                let mut data = vec![0; len];
                cursor.read_exact(&mut data)?;
                Ok(QueryResponse {
                    connection_id,
                    data,
                })
            }
            PROXY_S2C_PACKET_ID => Ok(ProxyS2CPacket {
                connection_id: cursor.read_u64::<BigEndian>()?,
                data: Self::read_remaining(cursor)?,
            }),
            PROXY_DISCONNECT_ID => Ok(ProxyDisconnect {
                connection_id: cursor.read_u64::<BigEndian>()?,
            }),
            REQUEST_DIRECT_JOIN_ID => Ok(RequestDirectJoin {
                connection_id: cursor.read_connection_id()?,
            }),
            NEW_QUERY_RESPONSE_ID => Ok(NewQueryResponse {
                connection_id: cursor.read_connection_id()?,
                data: Self::read_remaining(cursor)?,
            }),
            REQUEST_PUNCH_OPEN_ID => Ok(RequestPunchOpen {
                target_connection: cursor.read_connection_id()?,
                purpose: cursor.read_string()?,
                punch_id: cursor.read_uuid()?,
                my_host: cursor.read_string()?,
                my_port: cursor.read_u16::<BigEndian>()?,
                my_local_host: cursor.read_string()?,
                my_local_port: cursor.read_u16::<BigEndian>()?,
            }),
            PUNCH_FAILED_ID => Ok(PunchFailed {
                target_connection: cursor.read_connection_id()?,
                punch_id: cursor.read_uuid()?,
            }),
            BEGIN_PORT_LOOKUP_ID => Ok(BeginPortLookup {
                lookup_id: cursor.read_uuid()?,
            }),
            PUNCH_SUCCESS_ID => Ok(PunchSuccess {
                connection_id: cursor.read_connection_id()?,
                punch_id: cursor.read_uuid()?,
                host: cursor.read_string()?,
                port: cursor.read_u16::<BigEndian>()?,
            }),
            _ => invalid_data!("Unknown message ID {id}"),
        }
    }

    fn read_uuid_vec(cursor: &mut Cursor<&[u8]>) -> io::Result<Vec<Uuid>> {
        cursor.read_vec(|c| c.read_uuid())
    }

    fn read_remaining(cursor: &mut Cursor<&[u8]>) -> io::Result<Vec<u8>> {
        let mut result = vec![0; cursor.remaining()];
        cursor.read_exact(&mut result)?;
        Ok(result)
    }
}

pub fn first_protocol_version(id: u8) -> Option<u32> {
    match id {
        LIST_ONLINE_ID => Some(2),
        FRIEND_REQUEST_ID => Some(2),
        PUBLISHED_WORLD_ID => Some(2),
        CLOSED_WORLD_ID => Some(2),
        REQUEST_JOIN_ID => Some(2),
        JOIN_GRANTED_ID => Some(2),
        QUERY_REQUEST_ID => Some(2),
        QUERY_RESPONSE_ID => Some(2),
        PROXY_S2C_PACKET_ID => Some(2),
        PROXY_DISCONNECT_ID => Some(2),
        REQUEST_DIRECT_JOIN_ID => Some(4),
        NEW_QUERY_RESPONSE_ID => Some(5),
        REQUEST_PUNCH_OPEN_ID => Some(7),
        PUNCH_FAILED_ID => Some(7),
        BEGIN_PORT_LOOKUP_ID => Some(7),
        PUNCH_SUCCESS_ID => Some(7),
        _ => None,
    }
}
