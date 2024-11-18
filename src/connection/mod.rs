use crate::connection::connection_id::ConnectionId;
use crate::country_code::CountryCode;
use crate::minecraft_crypt::Aes128Cfb;
use crate::socket_wrapper::SocketWrapper;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub mod connection_id;
pub mod connection_set;

#[derive(Clone)]
pub struct Connection {
    pub id: ConnectionId,
    pub addr: IpAddr,
    pub user_uuid: Uuid,
    pub live: Arc<Mutex<LiveConnection>>,
}

pub struct LiveConnection {
    pub socket: SocketWrapper,
    pub country: Option<CountryCode>,
    pub open: bool,
    pub encrypt_cipher: Option<Aes128Cfb>,
    pub decrypt_cipher: Option<Aes128Cfb>,
}
