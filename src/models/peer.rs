use super::{
    common::{IpType, Port},
    tracker::AnnounceRequest,
};
use crate::config::TrackerConfig;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    ops,
};
use ts_utils::time::{Clock, Duration};

/// An enumeration representing the type of a peer in a BitTorrent swarm.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PeerType {
    /// A leecher is a peer that is currently downloading the torrent and does not have the complete file.
    Leecher,

    /// A seeder is a peer that has the entire content of the torrent and can upload data to other peers.
    Seeder,

    /// A partial seed is a peer that is incomplete without downloading anything more.
    /// More information: https://www.bittorrent.org/beps/bep_0021.html
    Partial,
}

/// Represents a peer in a BitTorrent swarm.
#[derive(Debug, Clone)]
pub struct Peer {
    /// The address of the peer.
    pub addr: PeerAddr,

    /// The duration since unix epoch at which the peer will expire.
    pub expire_at: Duration,
}

impl Peer {
    pub fn ip_type(&self) -> IpType {
        self.addr.ip_type()
    }
}

impl From<(&AnnounceRequest, IpAddr, &TrackerConfig)> for Peer {
    fn from(value: (&AnnounceRequest, IpAddr, &TrackerConfig)) -> Self {
        let (req, ip, config) = value;
        let expire_at = Clock::now_since_epoch() + Duration::from(config.peer_idle_time);

        let addr = match ip {
            IpAddr::V4(ip) => (ip, req.port).into(),
            IpAddr::V6(ip) => (ip, req.port).into(),
        };

        Self { addr, expire_at }
    }
}

pub const PEER_ADDR_V4_LENGTH: usize = 6;
pub const PEER_ADDR_V6_LENGTH: usize = 18;
pub const IP_V4_LENGTH: usize = 4;
pub const IP_V6_LENGTH: usize = 16;
pub const PORT_LENGTH: usize = 2;

/// The IP address and port number of a peer in a BitTorrent swarm.
#[derive(PartialEq, Eq, Clone)]
pub enum PeerAddr {
    /// Represents an IPv4 address of a peer. It contains an array of `u8` with a length of `PEER_ADDR_V4_LENGTH`,
    /// which typically represents the IPv4 address in octets (4 bytes) and an additional 2 bytes for the port number.
    V4(PeerAddrV4),

    /// Represents an IPv6 address of a peer. It contains an array of `u8` with a length of `PEER_ADDR_V6_LENGTH`,
    /// which typically represents the IPv6 address in octets (16 bytes) and an additional 2 bytes for the port number.
    V6(PeerAddrV6),
}

impl PeerAddr {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            PeerAddr::V4(addr) => addr.as_ref(),
            PeerAddr::V6(addr) => addr.as_ref(),
        }
    }

    pub fn ip_type(&self) -> IpType {
        match self {
            PeerAddr::V4(_) => IpType::V4,
            PeerAddr::V6(_) => IpType::V6,
        }
    }
}

macro_rules! impl_ip_port_into_peer_addr {
    ($ip:ty, $into:ident, $type:expr, $length:expr, $ip_length:expr) => {
        impl From<($ip, Port)> for PeerAddr {
            fn from(value: ($ip, Port)) -> Self {
                let (ip, port) = value;
                let mut bytes = [0; $length];

                bytes[..$ip_length].copy_from_slice(&ip.octets());
                bytes[$ip_length..].copy_from_slice(&port.0.to_be_bytes());
                PeerAddr::$into($type(bytes))
            }
        }
    };
}

impl_ip_port_into_peer_addr!(Ipv4Addr, V4, PeerAddrV4, PEER_ADDR_V4_LENGTH, IP_V4_LENGTH);
impl_ip_port_into_peer_addr!(Ipv6Addr, V6, PeerAddrV6, PEER_ADDR_V6_LENGTH, IP_V6_LENGTH);

macro_rules! extract_ip_port {
    ($ip_type:expr, $ip_conv:ty, $ip_length:expr, $bytes:expr) => {
        let mut ip_bytes = [0; $ip_length];
        ip_bytes.copy_from_slice(&$bytes[..$ip_length]);
        let ip = $ip_type(<$ip_conv>::from(ip_bytes));

        let mut port_bytes = [0; PORT_LENGTH];
        port_bytes.copy_from_slice(&$bytes[$ip_length..]);
        let port = u16::from_be_bytes(port_bytes);

        return (ip, Port(port));
    };
}

impl Into<(IpAddr, Port)> for &PeerAddr {
    fn into(self) -> (IpAddr, Port) {
        match self {
            PeerAddr::V4(bytes) => {
                extract_ip_port!(IpAddr::V4, std::net::Ipv4Addr, IP_V4_LENGTH, bytes);
            }
            PeerAddr::V6(bytes) => {
                extract_ip_port!(IpAddr::V6, std::net::Ipv6Addr, IP_V6_LENGTH, bytes);
            }
        }
    }
}

impl Into<SocketAddr> for &PeerAddr {
    fn into(self) -> SocketAddr {
        let (ip, port) = self.into();
        SocketAddr::new(ip, port.0)
    }
}

impl fmt::Display for PeerAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Into::<SocketAddr>::into(self))
    }
}

impl fmt::Debug for PeerAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<'de> Deserialize<'de> for PeerAddr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
        match bytes.len() {
            PEER_ADDR_V4_LENGTH => Ok(PeerAddr::V4(PeerAddrV4(bytes.try_into().unwrap()))),
            PEER_ADDR_V6_LENGTH => Ok(PeerAddr::V6(PeerAddrV6(bytes.try_into().unwrap()))),
            _ => Err(serde::de::Error::custom(format!(
                "invalid peer address length: {}",
                bytes.len()
            ))),
        }
    }
}

impl Serialize for PeerAddr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(self.as_bytes())
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct PeerAddrV4([u8; PEER_ADDR_V4_LENGTH]);

impl ops::Deref for PeerAddrV4 {
    type Target = [u8; PEER_ADDR_V4_LENGTH];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct PeerAddrV6([u8; PEER_ADDR_V6_LENGTH]);

impl ops::Deref for PeerAddrV6 {
    type Target = [u8; PEER_ADDR_V6_LENGTH];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<&[u8]> for PeerAddr {
    type Error = &'static str;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match value.len() {
            PEER_ADDR_V4_LENGTH => Ok(PeerAddr::V4(PeerAddrV4(value.try_into().unwrap()))),
            PEER_ADDR_V6_LENGTH => Ok(PeerAddr::V6(PeerAddrV6(value.try_into().unwrap()))),
            _ => Err("invalid peer address length"),
        }
    }
}
