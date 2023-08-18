use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};
use ts_utils::{hex, query};

use crate::constants;

/// The length of the Peer ID used in the BitTorrent protocol.
/// It is a constant of type usize, which represents the number of bytes in the Peer ID.
pub const PEER_ID_LENGTH: usize = 20;

/// The length of the Infohash used in the BitTorrent protocol.
/// It is a constant of type usize, which represents the number of bytes in the Infohash.
pub const INFOHASH_LENGTH: usize = 20;

/// Represents the information hash used in peer-to-peer (P2P) communication.
/// The InfoHash is a 20-byte fixed-size array that uniquely identifies a torrent file or resource.
#[derive(PartialEq, Eq, Hash, Clone, Default)]
pub struct InfoHash(pub [u8; INFOHASH_LENGTH]);

impl AsRef<[u8]> for InfoHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for InfoHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}

impl fmt::Debug for InfoHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Serialize for InfoHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let info_hash_str = hex::encode(self.as_ref());
        serializer.serialize_str(&info_hash_str)
    }
}

impl<'de> Deserialize<'de> for InfoHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <[u8; INFOHASH_LENGTH]>::try_from(query::deserialize_bytes(deserializer)?)
            .map_err(|_| serde::de::Error::custom(constants::TRACKER_ERROR_INVALID_INFOHASH))
            .map(InfoHash)
    }
}

impl std::convert::From<[u8; INFOHASH_LENGTH]> for InfoHash {
    fn from(val: [u8; INFOHASH_LENGTH]) -> Self {
        InfoHash(val)
    }
}

impl std::str::FromStr for InfoHash {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s)?;
        <[u8; INFOHASH_LENGTH]>::try_from(bytes.as_slice())
            .map_err(|_| hex::FromHexError::InvalidStringLength)
            .map(InfoHash)
    }
}

impl Ord for InfoHash {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl std::cmp::PartialOrd<InfoHash> for InfoHash {
    fn partial_cmp(&self, other: &InfoHash) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Represents a network port number.
/// A Port is a 16-bit unsigned integer used to specify a particular communication endpoint.
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct Port(pub u16);

impl fmt::Display for Port {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for Port {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

// Implement Serialize and Deserialize for NumOfBytes
impl Serialize for Port {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Port {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u16::deserialize(deserializer)?;
        Ok(Port(value))
    }
}

/// Represents a unique identifier for a peer in a peer-to-peer network.
/// A string of length 20 which this downloader uses as its id. Each downloader generates its own id at random at the start of a new download.
#[derive(PartialEq, Default, Eq, Hash, Clone)]
pub struct PeerId(pub [u8; PEER_ID_LENGTH]);

impl AsRef<[u8]> for PeerId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let peer_id_str = String::from_utf8_lossy(&self.0);
        write!(f, "{}", peer_id_str)
    }
}

impl fmt::Debug for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Serialize for PeerId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for PeerId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <[u8; PEER_ID_LENGTH]>::try_from(query::deserialize_bytes(deserializer)?)
            .map_err(|_| serde::de::Error::custom(constants::TRACKER_ERROR_INVALID_PEERID))
            .map(PeerId)
    }
}

impl std::convert::From<[u8; PEER_ID_LENGTH]> for PeerId {
    fn from(val: [u8; PEER_ID_LENGTH]) -> Self {
        PeerId(val)
    }
}

impl std::convert::TryFrom<&[u8]> for PeerId {
    type Error = ();

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        <[u8; PEER_ID_LENGTH]>::try_from(value)
            .map(PeerId)
            .map_err(|_| ())
    }
}

/// The length of the Peer Key used in the BitTorrent protocol.
pub const PEERKEY_LENGTH: usize = 4;

/// An additional identification that is intended to allow a client to prove their identity
/// should their IP address change. The key has at least 32bits worth of entropy.
/// https://www.bittorrent.org/beps/bep_0007.html
#[derive(PartialEq, Default, Eq, Hash, Clone)]
pub struct PeerKey(pub Option<[u8; PEERKEY_LENGTH]>);

impl std::ops::Deref for PeerKey {
    type Target = Option<[u8; PEERKEY_LENGTH]>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for PeerKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v: Option<&str> = Option::deserialize(deserializer)?;

        // Try to convert the string to a [u8; 4]
        let v: Option<[u8; PEERKEY_LENGTH]> = v.and_then(|v| {
            hex::decode(v)
                .ok()
                .and_then(|v| v.try_into().ok())
                .or_else(|| {
                    if v.len() >= PEERKEY_LENGTH {
                        v.as_bytes()[..PEERKEY_LENGTH].try_into().ok()
                    } else {
                        None
                    }
                })
        });

        Ok(PeerKey(v))
    }
}

impl Serialize for PeerKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            Some(v) => serializer.serialize_str(&hex::encode(v.as_ref())),
            None => serializer.serialize_none(),
        }
    }
}

impl fmt::Display for PeerKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.map(|v| u32::from_le_bytes(v)))
    }
}

impl fmt::Debug for PeerKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Into<Option<u32>> for PeerKey {
    fn into(self) -> Option<u32> {
        self.map(|v| u32::from_le_bytes(v))
    }
}

/// Represents the number of bytes.
/// The `NumOfBytes` struct encapsulates a 64-bit unsigned integer representing the count of bytes.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Default)]
pub struct NumOfBytes(pub u64);

impl fmt::Display for NumOfBytes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for NumOfBytes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

// Implement Serialize and Deserialize for NumOfBytes
impl Serialize for NumOfBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for NumOfBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        Ok(NumOfBytes(value))
    }
}

/// The `UnixEpochSecs` type represents a point in time measured as the number of seconds elapsed
/// since the Unix epoch (1970-01-01 00:00:00 UTC).
pub type UnixEpochSecs = u64;
pub type IntervalDuration = u32;

/// Represents an IP address type, which can be either V4 (IPv4) or V6 (IPv6).
/// This enum is used to differentiate between the two types of IP addresses.
#[derive(Debug, PartialEq, Serialize, Clone, Copy)]
pub enum IpType {
    V4, // IPv4 address type
    V6, // IPv6 address type
}
