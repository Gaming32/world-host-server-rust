use crate::invalid_data;
use std::io;
use std::io::{Cursor, Read};
use tokio::io::AsyncReadExt;
use tokio_util::bytes::Buf;

const VARINT_SEGMENT_BITS: i32 = 0x7f;
const VARINT_CONTINUE_BIT: i32 = 0x80;

pub trait MinecraftPacketAsyncRead {
    async fn read_var_int(&mut self) -> io::Result<i32>;
}

impl<T: AsyncReadExt + Unpin> MinecraftPacketAsyncRead for T {
    async fn read_var_int(&mut self) -> io::Result<i32> {
        let mut value = 0;
        let mut position = 0;

        loop {
            let current = self.read_u8().await? as i32;
            value |= (current & VARINT_SEGMENT_BITS) << position;

            if (current & VARINT_CONTINUE_BIT) == 0 {
                break;
            }

            position += 7;

            if position >= 32 {
                invalid_data!("VarInt is too big");
            }
        }

        Ok(value)
    }
}

pub trait MinecraftPacketRead {
    fn get_var_int(&mut self) -> io::Result<i32>;

    fn get_mc_string(&mut self, max_length: usize) -> io::Result<String>;
}

impl MinecraftPacketRead for Cursor<&[u8]> {
    fn get_var_int(&mut self) -> io::Result<i32> {
        let mut value = 0;
        let mut position = 0;

        loop {
            let current = self.get_u8() as i32;
            value |= (current & VARINT_SEGMENT_BITS) << position;

            if (current & VARINT_CONTINUE_BIT) == 0 {
                break;
            }

            position += 7;

            if position >= 32 {
                invalid_data!("VarInt is too big");
            }
        }

        Ok(value)
    }

    fn get_mc_string(&mut self, max_length: usize) -> io::Result<String> {
        let length = self.get_var_int()? as usize;
        if length > max_length {
            invalid_data!("String exceeds max_length ({max_length} bytes)");
        }
        let mut result = vec![0; length];
        Read::read_exact(self, &mut result)?;
        String::from_utf8(result).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

pub trait MinecraftPacketWrite {
    fn write_var_int(&mut self, value: i32) -> io::Result<()>;

    fn write_mc_string(&mut self, value: String, max_length: usize) -> io::Result<()>;
}

impl MinecraftPacketWrite for Vec<u8> {
    fn write_var_int(&mut self, mut value: i32) -> io::Result<()> {
        loop {
            if (value & !VARINT_SEGMENT_BITS) == 0 {
                self.push(value as u8);
                break;
            }

            self.push(((value & VARINT_SEGMENT_BITS) | VARINT_CONTINUE_BIT) as u8);

            value >>= 7;
        }
        Ok(())
    }

    fn write_mc_string(&mut self, value: String, max_length: usize) -> io::Result<()> {
        if value.len() > max_length {
            invalid_data!("String exceeds max_length ({max_length} bytes)");
        }
        self.write_var_int(value.len() as i32)?;
        self.extend_from_slice(value.as_bytes());
        Ok(())
    }
}
