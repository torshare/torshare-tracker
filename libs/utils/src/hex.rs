use hex::ToHex;
use serde::Serializer;

#[cfg_attr(feature = "coverage", inline(never))]
#[cfg_attr(not(feature = "coverage"), inline(always))]
pub fn encode(bin: &[u8]) -> String {
    bin.encode_hex::<String>()
}

#[cfg_attr(feature = "coverage", inline(never))]
#[cfg_attr(not(feature = "coverage"), inline(always))]
pub fn decode(hex: &str) -> Result<Vec<u8>, hex::FromHexError> {
    hex::decode(hex)
}

#[cfg_attr(feature = "coverage", inline(never))]
#[cfg_attr(not(feature = "coverage"), inline)]
pub fn serialize<T: AsRef<[u8]>, S>(bytes: T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let str = encode(bytes.as_ref());
    serializer.serialize_str(&str)
}
