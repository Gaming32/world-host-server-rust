use crate::serialization::serializable::PacketSerializable;

pub trait FieldedSerializer {
    fn fields(&self) -> Vec<Box<dyn PacketSerializable + '_>>;
}

impl<T: FieldedSerializer> PacketSerializable for T {
    fn serialize_to(&self, buf: &mut Vec<u8>) {
        for field in self.fields() {
            field.serialize_to(buf);
        }
    }
}

#[macro_export]
macro_rules! serial_fields {
    ($($x:expr),+ $(,)?) => (
        vec![$(std::boxed::Box::new(*$x)),+]
    );
}
