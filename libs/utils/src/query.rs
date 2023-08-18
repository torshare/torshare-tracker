use percent_encoding::percent_decode;
use serde::de::value::MapDeserializer;
use serde::de::Error as de_Error;
use serde::de::{self, IntoDeserializer};
use serde::forward_to_deserialize_any;
use std::borrow::Cow;

#[doc(inline)]
pub use serde::de::value::Error;

/// Create and return an instance of `UrlEncodedParse` with the provided byte slice as the `input` field.
pub fn parse(input: &[u8]) -> UrlEncodedParse<'_> {
    UrlEncodedParse { input }
}

/// Deserializes a `application/x-www-form-urlencoded` value from a `&[u8]`.
///
/// ```
/// let meal = vec![
///     ("bread".to_owned(), "baguette".to_owned()),
///     ("cheese".to_owned(), "comté".to_owned()),
///     ("meat".to_owned(), "ham".to_owned()),
///     ("fat".to_owned(), "butter".to_owned()),
/// ];
///
/// assert_eq!(
///     ts_utils::query::from_bytes::<Vec<(String, String)>>(
///         b"bread=baguette&cheese=comt%C3%A9&meat=ham&fat=butter"),
///     Ok(meal));
/// ```
pub fn from_bytes<'de, T>(input: &'de [u8]) -> Result<T, Error>
where
    T: de::Deserialize<'de>,
{
    T::deserialize(Deserializer::new(parse(input)))
}

/// A deserializer for the `application/x-www-form-urlencoded` format.
pub struct Deserializer<'de> {
    inner: MapDeserializer<'de, PartIterator<'de>, Error>,
}

impl<'de> Deserializer<'de> {
    /// Returns a new `Deserializer`.
    pub fn new(parser: UrlEncodedParse<'de>) -> Self {
        Deserializer {
            inner: MapDeserializer::new(PartIterator(parser)),
        }
    }
}

impl<'de> de::Deserializer<'de> for Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_map(self.inner)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_seq(self.inner)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.inner.end()?;
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool
        u8
        u16
        u32
        u64
        i8
        i16
        i32
        i64
        f32
        f64
        char
        str
        string
        option
        bytes
        byte_buf
        unit_struct
        newtype_struct
        tuple_struct
        struct
        identifier
        tuple
        enum
        ignored_any
    }
}

struct PartIterator<'de>(UrlEncodedParse<'de>);

impl<'de> Iterator for PartIterator<'de> {
    type Item = (Part<'de>, Part<'de>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| {
            (
                Part(PartType::Key(decode_utf8_lossy(k))),
                Part(PartType::Value(v)),
            )
        })
    }
}

enum PartType<'de> {
    Key(Cow<'de, str>),
    Value(Cow<'de, [u8]>),
}

struct Part<'de>(PartType<'de>);

impl<'de> Part<'de> {
    fn as_str(self) -> Cow<'de, str> {
        match self.0 {
            PartType::Value(value) => decode_utf8_lossy(value),
            PartType::Key(value) => value,
        }
    }
}

impl<'de> IntoDeserializer<'de> for Part<'de> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

macro_rules! forward_parsed_value {
    ($($ty:ident => $method:ident,)*) => {
        $(
            fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where V: de::Visitor<'de>
            {
                match self.as_str().parse::<$ty>() {
                    Ok(val) => val.into_deserializer().$method(visitor),
                    Err(e) => Err(de::Error::custom(e))
                }
            }
        )*
    }
}

/// A deserializer for the `application/x-www-form-urlencoded` key.
impl<'de> de::Deserializer<'de> for Part<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.as_str() {
            Cow::Borrowed(value) => visitor.visit_borrowed_str(value),
            Cow::Owned(value) => visitor.visit_string(value),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            PartType::Value(value) => match value {
                Cow::Borrowed(value) => visitor.visit_borrowed_bytes(value),
                Cow::Owned(value) => visitor.visit_byte_buf(value),
            },
            PartType::Key(_) => self.deserialize_any(visitor),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_enum(ValueEnumAccess(self.as_str()))
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }

    forward_to_deserialize_any! {
        char
        str
        string
        unit
        unit_struct
        struct
        identifier
        ignored_any
        map
        tuple_struct
    }

    forward_parsed_value! {
        bool => deserialize_bool,
        u8 => deserialize_u8,
        u16 => deserialize_u16,
        u32 => deserialize_u32,
        u64 => deserialize_u64,
        i8 => deserialize_i8,
        i16 => deserialize_i16,
        i32 => deserialize_i32,
        i64 => deserialize_i64,
        f32 => deserialize_f32,
        f64 => deserialize_f64,
    }
}

struct ValueEnumAccess<'de>(Cow<'de, str>);

impl<'de> de::EnumAccess<'de> for ValueEnumAccess<'de> {
    type Error = Error;
    type Variant = UnitOnlyVariantAccess;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(self.0.into_deserializer())?;
        Ok((variant, UnitOnlyVariantAccess))
    }
}

struct UnitOnlyVariantAccess;

impl<'de> de::VariantAccess<'de> for UnitOnlyVariantAccess {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        Err(Error::custom("expected unit variant"))
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::custom("expected unit variant"))
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::custom("expected unit variant"))
    }
}

/// The return type of `parse()`.
#[derive(Copy, Clone)]
pub struct UrlEncodedParse<'a> {
    input: &'a [u8],
}

impl<'a> Iterator for UrlEncodedParse<'a> {
    type Item = (Cow<'a, [u8]>, Cow<'a, [u8]>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.input.is_empty() {
                return None;
            }

            let mut split2 = self.input.splitn(2, |&b| b == b'&');
            let sequence = split2.next().unwrap();
            self.input = split2.next().unwrap_or(&[][..]);

            if sequence.is_empty() {
                continue;
            }

            let mut split2 = sequence.splitn(2, |&b| b == b'=');
            let name = split2.next().unwrap();
            let value = split2.next().unwrap_or(&[][..]);

            return Some((decode(name), decode(value)));
        }
    }
}

/// Replace b'+' with b' '
fn replace_plus(input: &[u8]) -> Cow<'_, [u8]> {
    match input.iter().position(|&b| b == b'+') {
        None => Cow::Borrowed(input),
        Some(first_position) => {
            let mut replaced = input.to_owned();
            replaced[first_position] = b' ';
            for byte in &mut replaced[first_position + 1..] {
                if *byte == b'+' {
                    *byte = b' ';
                }
            }
            Cow::Owned(replaced)
        }
    }
}

fn decode(input: &[u8]) -> Cow<[u8]> {
    let replaced = replace_plus(input);
    match percent_decode(&replaced).into() {
        Cow::Owned(vec) => Cow::Owned(vec),
        Cow::Borrowed(_) => replaced,
    }
}

fn decode_utf8_lossy(input: Cow<'_, [u8]>) -> Cow<'_, str> {
    // Note: This function is copied from the `percent_encoding` crate.
    match input {
        Cow::Borrowed(bytes) => String::from_utf8_lossy(bytes),
        Cow::Owned(bytes) => {
            match String::from_utf8_lossy(&bytes) {
                Cow::Borrowed(utf8) => {
                    // If from_utf8_lossy returns a Cow::Borrowed, then we can
                    // be sure our original bytes were valid UTF-8. This is because
                    // if the bytes were invalid UTF-8 from_utf8_lossy would have
                    // to allocate a new owned string to back the Cow so it could
                    // replace invalid bytes with a placeholder.

                    // First we do a debug_assert to confirm our description above.
                    let raw_utf8: *const [u8] = utf8.as_bytes();
                    debug_assert!(raw_utf8 == &*bytes as *const [u8]);

                    // Given we know the original input bytes are valid UTF-8,
                    // and we have ownership of those bytes, we re-use them and
                    // return a Cow::Owned here.
                    Cow::Owned(unsafe { String::from_utf8_unchecked(bytes) })
                }
                Cow::Owned(s) => Cow::Owned(s),
            }
        }
    }
}

struct BytesVisitor;

impl<'de> serde::de::Visitor<'de> for BytesVisitor {
    type Value = Vec<u8>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a byte array")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.to_vec())
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v)
    }
}

/// Custom deserialization function for converting bytes to vec.
pub fn deserialize_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_bytes(BytesVisitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default, PartialEq, serde::Deserialize)]
    struct AnnounceRequest {
        #[serde(deserialize_with = "deserialize_bytes")]
        info_hash: Vec<u8>,
        #[serde(deserialize_with = "deserialize_bytes")]
        peer_id: Vec<u8>,
        port: u16,
        uploaded: u64,
        downloaded: u64,
        left: u64,
        event: String,
    }

    #[test]
    fn test_from_bytes() {
        let input = b"info_hash=%07%D1W%EF%87%19%80%F8%08%E27%20%E5S%D7%19hB%E8%A1&peer_id=1234567890ABCDEFGHIJ&port=6881&uploaded=123456789&downloaded=987654321&left=1234567890&event=started";

        let info_hash = hex::decode("07d157ef871980f808e23720e553d7196842e8a1").unwrap();
        let peer_id = "1234567890ABCDEFGHIJ".as_bytes().try_into().unwrap();

        let expected_req = AnnounceRequest {
            info_hash,
            peer_id,
            port: 6881,
            uploaded: 123456789,
            downloaded: 987654321,
            left: 1234567890,
            event: "started".to_string(),
        };

        let result: Result<AnnounceRequest, _> = from_bytes(input);
        assert!(result.is_ok());

        let req = result.unwrap();
        assert_eq!(req, expected_req);
    }
}
