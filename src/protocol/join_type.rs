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
}
