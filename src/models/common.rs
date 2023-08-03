use bip_bencode::BencodeMut;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, time::Duration};
use ts_utils::hex;

/// The length of the Peer ID used in the BitTorrent protocol.
/// It is a constant of type usize, which represents the number of bytes in the Peer ID.
const PEER_ID_LENGTH: usize = 20;

/// The length of the Infohash used in the BitTorrent protocol.
/// It is a constant of type usize, which represents the number of bytes in the Infohash.
const INFOHASH_LENGTH: usize = 20;

/// Represents an IP address type, which can be either V4 (IPv4) or V6 (IPv6).
/// This enum is used to differentiate between the two types of IP addresses.
#[derive(Debug, PartialEq, Serialize)]
pub enum IpType {
    V4, // IPv4 address type
    V6, // IPv6 address type
}

/// Represents the information hash used in peer-to-peer (P2P) communication.
/// The InfoHash is a 20-byte fixed-size array that uniquely identifies a torrent file or resource.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Default)]
pub struct InfoHash(pub [u8; INFOHASH_LENGTH]);

impl AsRef<[u8]> for InfoHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Display for InfoHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

impl std::fmt::Debug for InfoHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

impl Serialize for InfoHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let info_hash_str = hex::encode(&self.0);
        serializer.serialize_str(&info_hash_str)
    }
}

impl<'de> Deserialize<'de> for InfoHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: [u8; INFOHASH_LENGTH] = Deserialize::deserialize(deserializer)?;
        Ok(InfoHash(bytes))
    }
}

/// Represents a network port number.
/// A Port is a 16-bit unsigned integer used to specify a particular communication endpoint.
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct Port(pub u16);

impl std::fmt::Display for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Debug for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
#[derive(PartialEq, Default, Eq, Hash, Clone, Copy)]
pub struct PeerId(pub [u8; PEER_ID_LENGTH]);

impl AsRef<[u8]> for PeerId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let peer_id_str = String::from_utf8_lossy(&self.0);
        write!(f, "{}", peer_id_str)
    }
}

impl std::fmt::Debug for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let peer_id_str = String::from_utf8_lossy(&self.0);
        write!(f, "{}", peer_id_str)
    }
}

impl Serialize for PeerId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let peer_id_str = String::from_utf8_lossy(&self.0);
        serializer.serialize_str(&peer_id_str)
    }
}

impl<'de> Deserialize<'de> for PeerId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: [u8; PEER_ID_LENGTH] = Deserialize::deserialize(deserializer)?;
        Ok(PeerId(bytes))
    }
}

/// Represents the number of bytes.
/// The `NumOfBytes` struct encapsulates a 64-bit unsigned integer representing the count of bytes.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Default)]
pub struct NumOfBytes(pub u64);

impl std::fmt::Display for NumOfBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Debug for NumOfBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
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

pub type EpochTime = Duration;
pub type IntervalTime = u32;

pub trait Ip: Clone + Copy + Debug + PartialEq + Eq {}

impl Ip for std::net::Ipv4Addr {}
impl Ip for std::net::Ipv6Addr {}

pub trait Bencode<'a> {
    fn bencode(&self) -> BencodeMut<'a>;
}

impl<'a> Bencode<'a> for IntervalTime {
    fn bencode(&self) -> BencodeMut<'a> {
        ben_int!(i64::from(*self))
    }
}
