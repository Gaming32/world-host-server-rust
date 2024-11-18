use crate::connection::connection_id::ConnectionId;
use byteorder::{BigEndian, ReadBytesExt};
use std::io;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

pub trait WHAsyncReadExt {
    async fn read_string(&mut self) -> io::Result<String>;

    async fn read_uuid(&mut self) -> io::Result<Uuid>;
}

impl<T: AsyncReadExt + Unpin> WHAsyncReadExt for T {
    async fn read_string(&mut self) -> io::Result<String> {
        let mut result = vec![0; self.read_u16().await? as usize];
        self.read_exact(&mut result).await?;
        String::from_utf8(result).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    async fn read_uuid(&mut self) -> io::Result<Uuid> {
        Ok(Uuid::from_u128(self.read_u128().await?))
    }
}

pub trait WHReadBytesExt {
    fn read_string(&mut self) -> io::Result<String>;

    fn read_uuid(&mut self) -> io::Result<Uuid>;

    fn read_connection_id(&mut self) -> io::Result<ConnectionId>;

    fn read_vec<V: Copy, F>(&mut self, reader: F) -> io::Result<Vec<V>>
    where
        F: Fn(&mut Self) -> io::Result<V>;
}

impl<T: ReadBytesExt> WHReadBytesExt for T {
    fn read_string(&mut self) -> io::Result<String> {
        let mut result = vec![0; self.read_u16::<BigEndian>()? as usize];
        self.read_exact(&mut result)?;
        String::from_utf8(result).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    fn read_uuid(&mut self) -> io::Result<Uuid> {
        Ok(Uuid::from_u128(self.read_u128::<BigEndian>()?))
    }

    fn read_connection_id(&mut self) -> io::Result<ConnectionId> {
        ConnectionId::new(self.read_u64::<BigEndian>()?)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    fn read_vec<V: Copy, F>(&mut self, reader: F) -> io::Result<Vec<V>>
    where
        F: Fn(&mut Self) -> io::Result<V>,
    {
        let len = self.read_u32::<BigEndian>()? as usize;
        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            result.push(reader(self)?);
        }
        Ok(result)
    }
}
