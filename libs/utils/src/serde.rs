use serde::Deserialize;
use serde::Deserializer;
use serde::Serializer;

/// Custom deserialization function for converting a boolean to an integer
#[cfg_attr(feature = "coverage", inline(never))]
#[cfg_attr(not(feature = "coverage"), inline(always))]
pub fn deserialize_bool_to_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match u8::deserialize(deserializer)? {
        0 => Ok(false),
        _ => Ok(true),
    }
}

pub fn serialize_byte_array_to_str<T: AsRef<[u8]>, S>(bytes: T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let str: std::borrow::Cow<'_, str> = String::from_utf8_lossy(bytes.as_ref());
    serializer.serialize_str(&str)
}
