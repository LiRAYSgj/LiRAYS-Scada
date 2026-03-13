use heed::{BoxedError, BytesDecode, BytesEncode};
use prost::Message;
use std::borrow::Cow;

pub struct Proto<T>(std::marker::PhantomData<T>);

// Encoder (Struct -> Bytes)
impl<'a, T: Message + 'a> BytesEncode<'a> for Proto<T> {
    type EItem = T;

    fn bytes_encode(item: &'a Self::EItem) -> Result<Cow<'a, [u8]>, BoxedError> {
        Ok(Cow::Owned(item.encode_to_vec()))
    }
}

// Decoder (Bytes -> Struct)
impl<'a, T: Message + Default + 'a> BytesDecode<'a> for Proto<T> {
    type DItem = T;

    fn bytes_decode(bytes: &'a [u8]) -> Result<Self::DItem, BoxedError> {
        T::decode(bytes).map_err(|e| Box::new(e) as BoxedError)
    }
}
