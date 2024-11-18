use crate::serialization::serializable::PacketSerializable;

pub trait FieldedSerializer {
    fn fields(&self) -> Vec<&(dyn PacketSerializable + '_)>;
}

impl<T: FieldedSerializer> PacketSerializable for T {
    fn serialize_to(&self, buf: &mut Vec<u8>) {
        for field in self.fields() {
            field.serialize_to(buf);
        }
    }
}
