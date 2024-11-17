use md5::Digest;
use uuid::Uuid;

// Reimplementation of Java's UUID.nameUUIDFromBytes
pub fn java_name_uuid_from_bytes(name: &[u8]) -> Uuid {
    let mut bytes = [0u8; 16];
    {
        let result = md5::Md5::digest(name);
        bytes.copy_from_slice(&result);
    }
    bytes[6] &= 0x0f;
    bytes[6] |= 0x30;
    bytes[8] &= 0x3f;
    bytes[8] |= 0x80;
    Uuid::from_bytes(bytes)
}
