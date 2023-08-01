use serde::Deserialize;
use serde::Serialize;
use std::time::Duration;

/// Represents an IP address type, which can be either V4 (IPv4) or V6 (IPv6).
/// This enum is used to differentiate between the two types of IP addresses.
#[derive(Debug, PartialEq, Serialize)]
pub enum IpType {
    V4, // IPv4 address type
    V6, // IPv6 address type
}

/// Represents the information hash used in peer-to-peer (P2P) communication.
/// The InfoHash is a 20-byte fixed-size array that uniquely identifies a torrent file or resource.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct InfoHash(pub [u8; 20]);

/// Represents a network port number.
/// A Port is a 16-bit unsigned integer used to specify a particular communication endpoint.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct Port(pub u16);

/// Represents a unique identifier for a peer in a peer-to-peer network.
/// A string of length 20 which this downloader uses as its id. Each downloader generates its own id at random at the start of a new download.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct PeerId(pub [u8; 20]);

/// Represents the number of bytes.
/// The `NumOfBytes` struct encapsulates a 64-bit unsigned integer representing the count of bytes.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct NumOfBytes(pub u64);

/// Represents the event type for announcing a download status to a BitTorrent tracker.
/// The `AnnounceEvent` enum is used to indicate the different states of the announcement.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AnnounceEvent {
    /// An announcement using `started` is sent when a download first begins.
    Started,

    /// Downloaders send an announcement using `stopped` when they cease downloading.
    Stopped,

    /// `completed` is sent when the download is complete.
    /// No `completed` is sent if the file was complete when `started`.
    Completed,

    /// `paused` is sent when the peer is a partial seed.
    /// See: http://www.bittorrent.org/beps/bep_0021.html#tracker-scrapes
    Paused,

    /// When request is one performed at regular intervals.
    None,
}

pub type EpochTime = Duration;
