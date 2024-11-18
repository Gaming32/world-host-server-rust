use crate::connection::connection_id::ConnectionId;
use crate::country_code::CountryCode;
use crate::json_data::ExternalProxy;
use crate::minecraft_crypt::Aes128Cfb;
use crate::protocol::protocol_versions;
use crate::protocol::s2c_message::WorldHostS2CMessage;
use crate::protocol::security::SecurityLevel;
use crate::socket_wrapper::SocketWrapper;
use std::io;
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
    pub protocol_version: u32,
    pub live: Arc<Mutex<LiveConnection>>,
}

pub struct LiveConnection {
    pub socket: SocketWrapper,
    pub country: Option<CountryCode>,
    pub external_proxy: Option<Arc<ExternalProxy>>,
    pub open: bool,
    pub encrypt_cipher: Option<Aes128Cfb>,
    pub decrypt_cipher: Option<Aes128Cfb>,
}

impl Connection {
    pub fn security_level(&self) -> SecurityLevel {
        SecurityLevel::from(
            self.user_uuid,
            self.protocol_version >= protocol_versions::NEW_AUTH_PROTOCOL,
        )
    }

    pub async fn send_message(&self, message: WorldHostS2CMessage) -> io::Result<()> {
        if self.protocol_version >= message.first_protocol() {
            self.live.lock().await.send_message(message).await
        } else {
            Ok(())
        }
    }

    pub async fn close_error(&self, message: String) {
        self.live.lock().await.close_error(message).await
    }
}

impl LiveConnection {
    async fn send_message(&mut self, message: WorldHostS2CMessage) -> io::Result<()> {
        self.socket
            .send_message(message, self.encrypt_cipher.as_mut())
            .await
    }

    async fn close_error(&mut self, message: String) {
        self.socket
            .close_error(message, self.encrypt_cipher.as_mut())
            .await
    }
}
