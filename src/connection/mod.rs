use crate::connection::connection_id::ConnectionId;
use crate::country_code::CountryCode;
use crate::json_data::ExternalProxy;
use crate::minecraft_crypt::Aes128Cfb;
use crate::protocol::c2s_message::WorldHostC2SMessage;
use crate::protocol::protocol_versions;
use crate::protocol::s2c_message::WorldHostS2CMessage;
use crate::protocol::security::SecurityLevel;
use crate::socket_wrapper::{SocketReadWrapper, SocketWriteWrapper};
use std::collections::HashSet;
use std::io;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub mod connection_id;
pub mod connection_set;

pub type Connection = Arc<ConnectionInfo>;

pub struct ConnectionInfo {
    pub id: ConnectionId,
    pub addr: IpAddr,
    pub user_uuid: Uuid,
    pub protocol_version: u32,
    pub state: Mutex<ConnectionState>,
    pub read: Mutex<ConnectionRead>,
    pub write: Mutex<ConnectionWrite>,
}

pub struct ConnectionState {
    pub country: Option<CountryCode>,
    pub external_proxy: Option<Arc<ExternalProxy>>,
    pub open_to_friends: HashSet<Uuid>,
}

pub struct ConnectionRead {
    pub socket: SocketReadWrapper,
    pub cipher: Option<Aes128Cfb>,
}

pub struct ConnectionWrite {
    pub socket: SocketWriteWrapper,
    pub cipher: Option<Aes128Cfb>,
}

impl ConnectionInfo {
    pub fn security_level(&self) -> SecurityLevel {
        SecurityLevel::from(
            self.user_uuid,
            self.protocol_version >= protocol_versions::NEW_AUTH_PROTOCOL,
        )
    }

    pub async fn recv_message(&self) -> io::Result<WorldHostC2SMessage> {
        self.read
            .lock()
            .await
            .recv_message(self.protocol_version)
            .await
    }

    pub async fn send_message(&self, message: &WorldHostS2CMessage) -> io::Result<()> {
        if self.protocol_version >= message.first_protocol() {
            self.write.lock().await.send_message(message).await
        } else {
            Ok(())
        }
    }

    pub async fn close_error(&self, message: String) {
        self.write.lock().await.close_error(message).await
    }
}

impl ConnectionRead {
    async fn recv_message(&mut self, protocol_version: u32) -> io::Result<WorldHostC2SMessage> {
        self.socket
            .recv_message(&mut self.cipher, Some(protocol_version))
            .await
    }
}

impl ConnectionWrite {
    async fn send_message(&mut self, message: &WorldHostS2CMessage) -> io::Result<()> {
        self.socket.send_message(message, &mut self.cipher).await
    }

    async fn close_error(&mut self, message: String) {
        self.socket.close_error(message, &mut self.cipher).await
    }
}
