use std::io::Write;
use std::net::IpAddr;
use uuid::Uuid;

pub trait PacketSerializable {
    fn serialize_to(&self, buf: &mut Vec<u8>);
}

impl PacketSerializable for bool {
    fn serialize_to(&self, buf: &mut Vec<u8>) {
        buf.push(*self as u8)
    }
}

impl PacketSerializable for u16 {
    fn serialize_to(&self, buf: &mut Vec<u8>) {
        buf.write_all(&self.to_be_bytes()).unwrap()
    }
}

impl PacketSerializable for u32 {
    fn serialize_to(&self, buf: &mut Vec<u8>) {
        buf.write_all(&self.to_be_bytes()).unwrap()
    }
}

impl PacketSerializable for u64 {
    fn serialize_to(&self, buf: &mut Vec<u8>) {
        buf.write_all(&self.to_be_bytes()).unwrap()
    }
}

impl PacketSerializable for Uuid {
    fn serialize_to(&self, buf: &mut Vec<u8>) {
        buf.write_all(self.as_bytes()).unwrap()
    }
}

impl PacketSerializable for Vec<u8> {
    fn serialize_to(&self, buf: &mut Vec<u8>) {
        buf.write_all(self).unwrap()
    }
}

impl PacketSerializable for String {
    fn serialize_to(&self, buf: &mut Vec<u8>) {
        buf.write_all(self.as_bytes()).unwrap()
    }
}

impl PacketSerializable for IpAddr {
    fn serialize_to(&self, buf: &mut Vec<u8>) {
        match self {
            IpAddr::V4(addr) => {
                buf.push(4);
                buf.write_all(&addr.octets()).unwrap()
            }
            IpAddr::V6(addr) => {
                buf.push(16);
                buf.write_all(&addr.octets()).unwrap()
            }
        }
    }
}
