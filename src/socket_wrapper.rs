use crate::invalid_data;
use crate::minecraft_crypt::Aes128Cfb;
use crate::protocol::c2s_message::WorldHostC2SMessage;
use crate::protocol::s2c_message::WorldHostS2CMessage;
use crate::serialization::serializable::PacketSerializable;
use cfb8::cipher::AsyncStreamCipher;
use log::warn;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf, ReadHalf, WriteHalf};
use tokio::net::TcpStream;

pub struct SocketReadWrapper(pub OwnedReadHalf);

pub struct SocketWriteWrapper(pub OwnedWriteHalf);

impl SocketReadWrapper {
    pub async fn recv_message(
        &mut self,
        decrypt_cipher: &mut Option<Aes128Cfb>,
        max_protocol_version: Option<u32>,
    ) -> io::Result<WorldHostC2SMessage> {
        let size = {
            let mut initial = [0; 4];
            self.0.read_exact(&mut initial).await?;
            if let Some(cipher) = decrypt_cipher {
                cipher.decrypt(&mut initial);
            }
            u32::from_be_bytes(initial) as usize
        };

        if size == 0 {
            invalid_data!("Message is empty");
        }

        if size > 2 * 1024 * 1024 {
            const SKIP_BUFFER_SIZE: usize = 2048;
            let mut skip_buf = [0; SKIP_BUFFER_SIZE];
            let mut remaining = size;
            while remaining > 0 {
                remaining -= self
                    .0
                    .read(&mut skip_buf[..remaining.min(SKIP_BUFFER_SIZE)])
                    .await?;
            }
            invalid_data!("Messages bigger than 2 MB are not allowed.");
        }

        let mut data = vec![0; size];
        self.0.read_exact(&mut data).await?;
        if let Some(cipher) = decrypt_cipher {
            cipher.decrypt(&mut data);
        }

        WorldHostC2SMessage::parse(data[0], &data[1..], max_protocol_version)
    }
}

impl SocketWriteWrapper {
    pub async fn send_message(
        &mut self,
        message: &WorldHostS2CMessage,
        encrypt_cipher: &mut Option<Aes128Cfb>,
    ) -> io::Result<()> {
        let mut buf = vec![message.type_id()];
        message.serialize_to(&mut buf);
        buf.splice(0..0, (buf.len() as u32).to_be_bytes());
        if let Some(cipher) = encrypt_cipher {
            cipher.encrypt(&mut buf);
        }
        self.0.write_all(&buf).await?;
        self.0.flush().await
    }

    pub async fn close_error(&mut self, message: String, encrypt_cipher: &mut Option<Aes128Cfb>) {
        if let Err(error) = self
            .send_message(
                &WorldHostS2CMessage::Error {
                    message,
                    critical: true,
                },
                encrypt_cipher,
            )
            .await
        {
            warn!("Error in critical error sending: {error}");
        }
        if let Err(error) = self.0.shutdown().await {
            warn!("Error shutting down socket: {error}");
        }
    }
}
