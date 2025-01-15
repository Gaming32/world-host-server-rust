use crate::connection::Connection;
use crate::protocol::s2c_message::WorldHostS2CMessage;
use crate::server_state::FullServerConfig;
use byteorder::{BigEndian, ReadBytesExt};
use std::io;
use std::io::Cursor;

#[derive(Clone, Debug)]
pub enum JoinType {
    UPnP(u16),
    Proxy,
    Punch,
}

impl JoinType {
    pub fn decode(cursor: &mut Cursor<&[u8]>) -> io::Result<JoinType> {
        use JoinType::*;
        let id = cursor.read_u8()?;
        match id {
            0 => Ok(UPnP(cursor.read_u16::<BigEndian>()?)),
            1 => Ok(Proxy),
            2 => Ok(Punch),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Received packet with unknown joinTypeId from client: {id}"),
            )),
        }
    }

    pub async fn to_online_game(
        &self,
        connection: &Connection,
        config: &FullServerConfig,
    ) -> Option<WorldHostS2CMessage> {
        match self {
            JoinType::UPnP(port) => Some(WorldHostS2CMessage::OnlineGame {
                host: connection.addr.to_string(),
                port: *port,
                owner_cid: connection.id,
            }),
            JoinType::Proxy => {
                let external_proxy = if connection.protocol_version >= 3 {
                    connection.state.lock().await.external_proxy.clone()
                } else {
                    None
                };

                let base_addr = external_proxy
                    .clone()
                    .and_then(|p| p.base_addr.clone())
                    .or_else(|| config.base_addr.clone())?;

                let port = external_proxy
                    .map(|p| p.mc_port)
                    .unwrap_or_else(|| config.ex_java_port);

                Some(WorldHostS2CMessage::OnlineGame {
                    host: format!("{}.{}", connection.id, base_addr),
                    port,
                    owner_cid: connection.id,
                })
            }
            JoinType::Punch => None,
        }
    }
}
