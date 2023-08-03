use crate::constants::{TRACKER_RESPONSE_IP, TRACKER_RESPONSE_PEER_ID, TRACKER_RESPONSE_PORT};

use super::common::{Bencode, EpochTime, Ip, IpType, NumOfBytes, PeerId, Port};
use bip_bencode::BencodeMut;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};

pub struct Peer {
    /// The unique identifier for a peer.
    pub id: PeerId,
    /// An additional identification that is intended to allow a client to prove their identity
    /// should their IP address change
    pub key: Bytes,
    /// The IP and port this peer is listening on
    pub addr: SocketAddr,
    /// The number of bytes peer has uploaded.
    pub uploaded: NumOfBytes,
    /// The number of bytes peer has downloaded.
    pub downloaded: NumOfBytes,
    /// The number of bytes are left to download.
    pub left: NumOfBytes,
    /// The last time when peer announced.
    pub last_announced: EpochTime,
}

impl Peer {
    /// Get the IP address of the peer.
    ///
    /// # Returns
    ///
    /// The `IpAddr` representing the IP address of the peer.
    pub fn ip(&mut self) -> IpAddr {
        self.addr.ip()
    }

    /// Get the IP address type (IPv4 or IPv6) of the peer.
    ///
    /// # Returns
    ///
    /// The `IpType` representing the IP address type of the peer.
    pub fn ip_type(&self) -> IpType {
        if self.addr.is_ipv4() {
            return IpType::V4;
        }

        IpType::V6
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ResponsePeer<I: Ip> {
    /// The peer's ID.
    pub peer_id: PeerId,
    /// The peer's IP address.
    pub ip_addr: I,
    /// The peer's port number.
    pub port: Port,
}

impl<'a, I: Ip + std::fmt::Display> Bencode<'a> for ResponsePeer<I> {
    fn bencode(&self) -> BencodeMut<'a> {
        ben_map! {
            TRACKER_RESPONSE_PEER_ID => ben_bytes!(self.peer_id.0.to_vec()),
            TRACKER_RESPONSE_IP => ben_bytes!(self.ip_addr.to_string()),
            TRACKER_RESPONSE_PORT => ben_int!(i64::from(self.port.0))
        }
    }
}
