use crate::minecraft_crypt::Aes128Cfb;
use crate::protocol::s2c_message::WorldHostS2CMessage;
use crate::serialization::serializable::PacketSerializable;
use cfb8::cipher::AsyncStreamCipher;
use log::warn;
use std::io;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

#[derive(Debug)]
pub struct SocketWrapper(pub TcpStream);

impl SocketWrapper {
    pub async fn send_message(
        &mut self,
        message: WorldHostS2CMessage,
        cipher: Option<&mut Aes128Cfb>,
    ) -> io::Result<()> {
        let mut buf = vec![message.type_id()];
        message.serialize_to(&mut buf);
        buf.splice(0..0, (buf.len() as u32).to_be_bytes());
        if let Some(cipher) = cipher {
            cipher.encrypt(&mut buf);
        }
        self.0.write_all(&buf).await?;
        self.0.flush().await
    }

    pub async fn send_close_error(&mut self, message: String, cipher: Option<&mut Aes128Cfb>) {
        if let Err(error) = self
            .send_message(
                WorldHostS2CMessage::Error {
                    message,
                    critical: true,
                },
                cipher,
            )
            .await
        {
            warn!("Error in critical error sending: {error}");
        }
    }

    pub async fn send_close_error_unencrypted(&mut self, message: String) {
        self.send_close_error(message, None).await
    }
}
