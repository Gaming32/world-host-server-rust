use crate::protocol::s2c_message::WorldHostS2CMessage;
use crate::serialization::serializable::PacketSerializable;
use log::warn;
use std::io;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

#[derive(Debug)]
pub struct SocketWrapper(pub TcpStream);

impl SocketWrapper {
    pub async fn send_message(&mut self, message: WorldHostS2CMessage) -> io::Result<()> {
        let mut buf = vec![message.type_id()];
        message.serialize_to(&mut buf);
        buf.splice(0..0, (buf.len() as u32).to_be_bytes());
        // TODO: Encryption
        self.0.write_all(&buf).await?;
        self.0.flush().await
    }

    pub async fn send_close_error(&mut self, message: String) {
        if let Err(error) = self
            .send_message(WorldHostS2CMessage::Error {
                message,
                critical: true,
            })
            .await
        {
            warn!("Error in critical error sending: {error}");
        }
    }
}
