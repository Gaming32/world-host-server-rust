use crate::connection::connection_id::ConnectionId;
use crate::country_code::CountryCode;
use crate::socket_wrapper::SocketWrapper;
use std::net::IpAddr;
use tokio::sync::Mutex;
use uuid::Uuid;

pub mod connection_id;
pub mod connection_set;

#[derive(Debug)]
pub struct Connection {
    pub id: ConnectionId,
    pub addr: IpAddr,
    pub user_uuid: Uuid,
    pub live: Mutex<LiveConnection>,
}

#[derive(Debug)]
pub struct LiveConnection {
    pub socket: SocketWrapper,
    pub country: Option<CountryCode>,
    pub open: bool,
}