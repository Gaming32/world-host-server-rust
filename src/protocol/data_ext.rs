use std::io;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

pub trait WHAsyncReadExt: AsyncReadExt + Unpin {
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
