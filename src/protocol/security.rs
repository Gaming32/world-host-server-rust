use crate::serialization::serializable::PacketSerializable;
use uuid::Uuid;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SecurityLevel {
    Insecure,
    Offline,
    Secure,
}

impl SecurityLevel {
    pub fn from(uuid: Uuid, secure_auth: bool) -> SecurityLevel {
        use SecurityLevel::*;
        if !secure_auth {
            Insecure
        } else if uuid.get_version_num() != 4 {
            Offline
        } else {
            Secure
        }
    }
}

impl PacketSerializable for SecurityLevel {
    fn serialize_to(&self, buf: &mut Vec<u8>) {
        buf.push(*self as u8)
    }
}
