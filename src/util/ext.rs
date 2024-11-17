use std::io;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

pub trait WHAsyncReadExt: AsyncReadExt + Unpin {
    async fn read_uuid(&mut self) -> io::Result<Uuid>;
}

impl<T: AsyncReadExt + Unpin> WHAsyncReadExt for T {
    async fn read_uuid(&mut self) -> io::Result<Uuid> {
        Ok(Uuid::from_u128(self.read_u128().await?))
    }
}
