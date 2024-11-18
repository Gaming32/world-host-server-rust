use crate::util::copy_to_fixed_size;
use md5::Digest;
use uuid::Uuid;

// Reimplementation of Java's UUID.nameUUIDFromBytes
pub fn java_name_uuid_from_bytes(name: &[u8]) -> Uuid {
    let mut bytes = copy_to_fixed_size(&md5::Md5::digest(name));
    bytes[6] &= 0x0f;
    bytes[6] |= 0x30;
    bytes[8] &= 0x3f;
    bytes[8] |= 0x80;
    Uuid::from_bytes(bytes)
}
