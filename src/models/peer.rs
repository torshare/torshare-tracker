use super::common::{EpochTime, IpType, NumOfBytes, PeerId};
use bytes::Bytes;
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
