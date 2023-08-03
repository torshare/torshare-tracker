use bip_bencode::{BMutAccess, BencodeMut};
use serde::{Deserialize, Serialize};
use ts_utils::{bencode, serde::deserialize_bool_to_int};

use super::{
    common::{Bencode, InfoHash, IntervalTime, NumOfBytes, PeerId, Port},
    peer::ResponsePeer,
};
use crate::constants;

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

impl From<u32> for AnnounceEvent {
    fn from(val: u32) -> Self {
        match val {
            2 => AnnounceEvent::Started,
            1 => AnnounceEvent::Completed,
            3 => AnnounceEvent::Stopped,
            _ => AnnounceEvent::None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnnounceRequest {
    /// The 20-byte SHA1 hash of the value of the info key from the Metainfo file
    pub info_hash: InfoHash,

    /// The port number that the client is listening on.
    pub port: Port,

    /// unique ID for the client, generated by the client at startup.
    /// example: `-AZ2060-`
    pub peer_id: PeerId,

    #[serde(default)]
    /// The total amount uploaded (since the client sent the `started` event to the tracker).
    pub uploaded: NumOfBytes,

    #[serde(default)]
    /// The total amount downloaded (since the client sent the `started` event to the tracker).
    pub downloaded: NumOfBytes,

    #[serde(default)]
    /// The number of bytes this peer still has to download.
    pub left: NumOfBytes,

    #[serde(default = "default_compact")]
    #[serde(deserialize_with = "deserialize_bool_to_int")]
    /// Setting this to `true` indicates that the client accepts a compact response.
    pub compact: bool,

    /// If specified, must be one of started, completed, stopped, (or empty which is the same as not being specified).
    /// If not specified, then this request is one performed at regular intervals.
    pub event: Option<AnnounceEvent>,

    /// Number of peers that the client would like to receive from the tracker.
    pub numwant: Option<u32>,
}

#[cfg_attr(feature = "coverage", inline(never))]
#[cfg_attr(not(feature = "coverage"), inline(always))]
fn default_compact() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Default)]
/// Represents the response sent by tracker after an "announce" request.
pub struct AnnounceResponse {
    #[serde(rename = "failure reason")]
    /// The value is a human-readable error message as to why the request failed.
    /// If present, the request has failed, and the other fields may not be valid.
    pub failure_reason: Option<String>,

    #[serde(rename = "warning message")]
    /// Similar to failure reason, but the response still gets processed normally.
    /// The warning message is shown just like an error.
    pub warning_message: Option<String>,

    /// Interval in seconds that the client should wait between sending regular requests to the tracker.
    pub interval: IntervalTime,

    #[serde(rename = "min interval")]
    /// Minimum announce interval. Clients must not reannounce more frequently than this.
    pub min_interval: IntervalTime,

    /// The number of peers with the entire file, aka "seeders".
    pub complete: u32,

    /// The number of non-seeder peers, aka "leechers".
    pub incomplete: u32,

    /// This list contains IPv4 addresses of peers that support the BitTorrent protocol over IPv4.
    /// It is optional and may be absent if there are no IPv4 peers in the response.
    pub peers: Option<Vec<ResponsePeer<std::net::Ipv4Addr>>>,

    /// This list contains IPv6 addresses of peers that support the BitTorrent protocol over IPv6.
    /// It is optional and may be absent if there are no IPv6 peers in the response.
    pub peers6: Option<Vec<ResponsePeer<std::net::Ipv6Addr>>>,
}

impl AnnounceResponse {
    /// Returns a compact representation of the `AnnounceResponse`.
    pub fn compact(&self) -> Vec<u8> {
        let message = (ben_map! {
            "lucky_number" => ben_int!(7),
            "lucky_string" => ben_bytes!("7")
        })
        .encode();

        println!(
            "message: {:?}",
            String::from_utf8_lossy(bencode::encode(&self).unwrap_or_default().as_ref())
        );

        return message;
    }

    /// Returns a non compact representation of the `AnnounceResponse`.
    pub fn non_compact(&self) -> Vec<u8> {
        let mut peers = BencodeMut::new_list();

        if let Some(peers_list) = peers.list_mut() {
            // ipv4 peers
            if let Some(peers) = &self.peers {
                for peer in peers {
                    peers_list.push(peer.bencode());
                }
            }

            // ipv6 peers
            if let Some(peers) = &self.peers6 {
                for peer in peers {
                    peers_list.push(peer.bencode());
                }
            }
        }

        let message = (ben_map! {
            constants::TRACKER_RESPONSE_COMPLETE => self.complete.bencode(),
            constants::TRACKER_RESPONSE_INCOMPLETE => self.incomplete.bencode(),
            constants::TRACKER_RESPONSE_INTERVAL => self.interval.bencode(),
            constants::TRACKER_RESPONSE_MIN_INTERVAL => self.min_interval.bencode(),
            constants::TRACKER_RESPONSE_PEERS => peers
        })
        .encode();

        return message;
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScrapeRequest {
    pub info_hash: Vec<InfoHash>,
}
