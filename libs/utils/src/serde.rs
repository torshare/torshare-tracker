use serde::Deserialize;
use serde::Deserializer;
use serde::Serializer;
use std::time::Duration;

/// Custom deserialization function for converting a boolean to an integer
pub fn deserialize_u8_to_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match u8::deserialize(deserializer)? {
        0 => Ok(false),
        _ => Ok(true),
    }
}

/// Custom serialization function for converting a byte array to a string.
pub fn serialize_byte_array_to_str<T: AsRef<[u8]>, S>(
    bytes: T,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let str: std::borrow::Cow<'_, str> = String::from_utf8_lossy(bytes.as_ref());
    serializer.serialize_str(&str)
}

/// Custom deserialization function for converting secs to `Duration`.
pub fn deserialize_secs_to_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let secs = u64::deserialize(deserializer)?;
    Ok(Duration::from_secs(secs))
}

/// Deserialize an optional string, treating an empty string as `None`.
pub fn deserialize_option_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<String> = Deserialize::deserialize(deserializer)?;
    match value {
        Some(value) => {
            if value.is_empty() {
                Ok(None)
            } else {
                Ok(Some(value))
            }
        }
        None => Ok(None),
    }
}

/// Deserialize an HTTP header name to lowercase.
pub fn deserialize_header_name<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_option_string(deserializer).map(|s| s.map(|s| s.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use serde::Serialize;
    use serde_test::{assert_de_tokens, assert_ser_tokens, Token};

    #[test]
    fn test_deserialize_u8_to_bool() {
        #[derive(Debug, PartialEq, Deserialize)]
        #[serde(transparent)]
        struct Struct {
            #[serde(deserialize_with = "deserialize_u8_to_bool")]
            value: bool,
        }

        assert_de_tokens(&Struct { value: true }, &[Token::U8(1)]);
        assert_de_tokens(&Struct { value: false }, &[Token::U8(0)]);
        assert_de_tokens(&Struct { value: true }, &[Token::U8(2)]);
    }

    #[test]
    fn test_deserialize_secs_to_duration() {
        #[derive(Debug, PartialEq, Deserialize)]
        #[serde(transparent)]
        struct Struct {
            #[serde(deserialize_with = "deserialize_secs_to_duration")]
            val: Duration,
        }

        assert_de_tokens(
            &Struct {
                val: Duration::from_secs(1),
            },
            &[Token::U8(1)],
        );

        assert_de_tokens(
            &Struct {
                val: Duration::from_secs(0),
            },
            &[Token::U8(0)],
        );

        assert_de_tokens(
            &Struct {
                val: Duration::from_secs(2),
            },
            &[Token::U8(2)],
        );
    }

    #[test]
    fn test_serialize_byte_array_to_str() {
        #[derive(Debug, PartialEq, Serialize)]
        #[serde(transparent)]
        struct Struct {
            #[serde(serialize_with = "serialize_byte_array_to_str")]
            val: Vec<u8>,
        }

        assert_ser_tokens(
            &Struct {
                val: vec![0x31, 0x32, 0x33],
            },
            &[Token::String("123")],
        );
    }

    #[test]
    fn test_deserialize_optional_string() {
        #[derive(Debug, PartialEq, Deserialize)]
        #[serde(transparent)]
        struct Struct {
            #[serde(deserialize_with = "deserialize_option_string")]
            val: Option<String>,
        }

        assert_de_tokens(
            &Struct {
                val: Some("123".to_string()),
            },
            &[Token::Some, Token::String("123")],
        );

        assert_de_tokens(&Struct { val: None }, &[Token::Some, Token::String("")]);
        assert_de_tokens(&Struct { val: None }, &[Token::None]);
    }

    #[test]
    fn test_deserialize_header_name() {
        #[derive(Debug, PartialEq, Deserialize)]
        #[serde(transparent)]
        struct Struct {
            #[serde(deserialize_with = "deserialize_header_name")]
            val: Option<String>,
        }

        assert_de_tokens(
            &Struct {
                val: Some("content-type".to_string()),
            },
            &[Token::Some, Token::String("Content-Type")],
        );

        assert_de_tokens(&Struct { val: None }, &[Token::None]);
    }
}
