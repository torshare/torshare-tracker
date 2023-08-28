use ahash::RandomState;
use bytes::BytesMut;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{
    common::{InfoHash, PeerId, PEER_ID_LENGTH},
    peer::{Peer, PeerType},
};

/// Represents a collection of peers, where each peer is identified by a `PeerIdKey`.
pub type PeerDict = IndexMap<PeerIdKey, Peer, RandomState>;

/// A type alias for a list of peers represented as a `Vec`.
pub type PeerList = Vec<(PeerIdKey, Peer)>;

/// Represents the swarm associated with a specific torrent in a BitTorrent tracker.
/// The swarm contains lists of different types of peers, as well as information about the torrent's completion status.
#[derive(Debug, Default)]
pub struct TorrentSwarm {
    /// A Map representing the seeders in the swarm.
    pub seeders: PeerDict,

    /// A Map representing the leechers in the swarm.
    pub leechers: PeerDict,

    /// A Map representing the partial seeders in the swarm.
    pub partial_seeds: PeerDict,
}

#[derive(Debug, Default, Clone)]
pub struct SwarmStats {
    pub complete: u32,
    pub incomplete: u32,
}

impl From<TorrentStats> for SwarmStats {
    fn from(stats: TorrentStats) -> Self {
        Self {
            complete: stats.seeders,
            incomplete: stats.incomplete,
        }
    }
}

macro_rules! update_peer_fields {
    ($peer:expr, $new_peer:expr) => {{
        let peer = $peer;
        let new_peer = $new_peer;

        peer.addr = new_peer.addr;
        peer.expire_at = new_peer.expire_at;
    }};
}

impl TorrentSwarm {
    /// Returns the number of seeders in the swarm.
    pub fn complete_count(&self) -> u32 {
        self.seeders.len() as u32
    }

    /// Returns the number of leechers in the swarm.
    pub fn incomplete_count(&self) -> u32 {
        (self.leechers.len() + self.partial_seeds.len()) as u32
    }

    pub fn insert_peer(
        &mut self,
        key: PeerIdKey,
        value: Peer,
        peer_type: PeerType,
    ) -> Option<Peer> {
        match peer_type {
            PeerType::Leecher => self.leechers.insert(key, value),
            PeerType::Seeder => self.seeders.insert(key, value),
            PeerType::Partial => self.partial_seeds.insert(key, value),
        }
    }

    pub fn get_peer(&self, key: &PeerIdKey, peer_type: PeerType) -> Option<&Peer> {
        match peer_type {
            PeerType::Leecher => self.leechers.get(key),
            _ => {
                if let Some(peer) = self.seeders.get(key) {
                    Some(peer)
                } else {
                    self.partial_seeds.get(key)
                }
            }
        }
    }

    pub fn get_mut_peer(&mut self, key: &PeerIdKey, peer_type: PeerType) -> Option<&mut Peer> {
        match peer_type {
            PeerType::Leecher => self.leechers.get_mut(key),
            _ => {
                if let Some(peer) = self.seeders.get_mut(key) {
                    Some(peer)
                } else {
                    self.partial_seeds.get_mut(key)
                }
            }
        }
    }

    pub fn promote_peer(&mut self, key: &PeerIdKey, peer: Peer) -> bool {
        if let Some(mut epeer) = self.remove_peer(key, PeerType::Leecher) {
            update_peer_fields!(&mut epeer, peer);
            self.insert_peer(key.clone(), epeer, PeerType::Seeder);
            return true;
        }

        false
    }

    pub fn update_or_insert_peer(&mut self, key: &PeerIdKey, peer: Peer, peer_type: PeerType) {
        match self.get_mut_peer(&key, peer_type) {
            Some(epeer) => update_peer_fields!(epeer, peer),
            None => {
                let _ = self.insert_peer(key.clone(), peer, peer_type);
            }
        };
    }

    pub fn remove_peer(&mut self, key: &PeerIdKey, peer_type: PeerType) -> Option<Peer> {
        match peer_type {
            PeerType::Leecher => self.leechers.remove(key),
            _ => {
                if let Some(peer) = self.seeders.remove(key) {
                    Some(peer)
                } else {
                    self.partial_seeds.remove(key)
                }
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Torrent {
    pub completed: u32,
}

impl Torrent {
    pub fn incr_completed(&mut self) {
        self.completed += 1;
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TorrentStats {
    /// The number of peers with the entire torrent.
    #[serde(rename = "complete")]
    pub seeders: u32,

    /// The total number of times the tracker has registered a completion for this torrent.
    #[serde(rename = "downloaded")]
    pub completed: u32,

    /// The number of non-seeder peers.
    pub incomplete: u32,
}

impl TorrentStats {
    pub fn new_with_completed(completed: u32) -> Self {
        Self {
            completed,
            seeders: 0,
            incomplete: 0,
        }
    }
}

pub type TorrentStatsList = Vec<(InfoHash, TorrentStats)>;

/// Represents a unique identifier for a peer in a collection of peers.
#[derive(Debug, PartialEq, Default, Eq, Hash, Clone)]
pub struct PeerIdKey(Vec<u8>);

impl PeerIdKey {
    pub fn new(peer_id: &PeerId, key: Option<&[u8]>) -> Self {
        let capacity = key.map(|k| k.len()).unwrap_or_default() + PEER_ID_LENGTH;
        let mut buf = BytesMut::with_capacity(capacity);

        buf.extend_from_slice(peer_id.as_ref());
        if let Some(key) = key {
            buf.extend_from_slice(key);
        }

        Self(buf.to_vec())
    }
}

impl Ord for PeerIdKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl std::cmp::PartialOrd<PeerIdKey> for PeerIdKey {
    fn partial_cmp(&self, other: &PeerIdKey) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl AsRef<[u8]> for PeerIdKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
