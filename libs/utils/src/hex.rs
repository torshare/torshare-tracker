use hex::ToHex;
use serde::Serializer;

#[inline]
pub fn encode(bin: &[u8]) -> String {
    bin.encode_hex::<String>()
}

#[inline]
pub fn decode(hex: &str) -> Result<Vec<u8>, hex::FromHexError> {
    hex::decode(hex)
}

#[inline]
pub fn serialize<T: AsRef<[u8]>, S>(bytes: T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let str = encode(bytes.as_ref());
    serializer.serialize_str(&str)
}
