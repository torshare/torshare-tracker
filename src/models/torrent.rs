use ahash::RandomState;
use bytes::BytesMut;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use ts_utils::shared::Shared;

use super::{
    common::{InfoHash, PeerId, PEER_ID_LENGTH},
    peer::{Peer, PeerType},
};

/// Represents a collection of peers, where each peer is identified by a `PeerIdHash`.
pub type PeerDict = IndexMap<PeerIdKey, Peer, RandomState>;

/// Represents the swarm associated with a specific torrent in a BitTorrent tracker.
/// The swarm contains lists of different types of peers, as well as information about the torrent's completion status.
#[derive(Debug, Default)]
pub struct TorrentSwarm {
    /// A Map representing the seeders in the swarm.
    seeders: PeerDict,

    /// A Map representing the leechers in the swarm.
    leechers: PeerDict,

    /// A Map representing the partial seeders in the swarm.
    partial_seeds: PeerDict,
}

macro_rules! update_peer_fields {
    ($peer:expr, $new_peer:expr) => {{
        let peer = $peer;
        let mut new_peer = $new_peer;

        peer.downloaded = new_peer.downloaded;
        peer.uploaded = new_peer.uploaded;
        peer.left = new_peer.left;
        peer.last_announce_at = new_peer.last_announce_at;

        if new_peer.addr_v4.is_some() {
            peer.addr_v4 = new_peer.addr_v4.take();
        }

        if new_peer.addr_v6.is_some() {
            peer.addr_v6 = new_peer.addr_v6.take();
        }
    }};
}

impl TorrentSwarm {
    /// Returns the number of seeders in the swarm.
    pub fn seeders_count(&self) -> usize {
        self.seeders.len()
    }

    /// Returns the number of leechers in the swarm.
    pub fn leechers_count(&self) -> usize {
        self.leechers.len()
    }

    /// Returns the number of peers in the swarm.
    /// This is the sum of the number of seeders and leechers.
    pub fn peers_count(&self) -> usize {
        self.seeders_count() + self.leechers_count()
    }

    /// Returns the number of partial seeders in the swarm.
    pub fn partial_seeds_count(&self) -> usize {
        self.partial_seeds.len()
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

    pub fn promote_peer(&mut self, key: &PeerIdKey, new_peer: Peer) -> bool {
        if let Some(mut peer) = self.remove_peer(key, PeerType::Leecher) {
            update_peer_fields!(&mut peer, new_peer);
            self.insert_peer(key.clone(), peer, PeerType::Seeder);
            return true;
        }

        false
    }

    pub fn update_or_insert_peer(
        &mut self,
        key: &PeerIdKey,
        new_peer: Peer,
        peer_type: PeerType,
    ) -> bool {
        match self.get_mut_peer(&key, peer_type) {
            Some(peer) => {
                update_peer_fields!(peer, new_peer);
                false
            }
            None => self.insert_peer(key.clone(), new_peer, peer_type).is_none(),
        }
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

    pub fn get_leechers(&self) -> &PeerDict {
        &self.leechers
    }

    pub fn get_seeders(&self) -> &PeerDict {
        &self.seeders
    }

    pub fn get_partial_seeds(&self) -> &PeerDict {
        &self.partial_seeds
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
    #[serde(rename = "incomplete")]
    pub leechers: u32,
}

impl TorrentStats {
    pub fn incr_completed(&mut self) {
        self.completed += 1;
    }
}

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

pub type TorrentSwarmDict = IndexMap<Shared<InfoHash>, TorrentSwarm, RandomState>;
pub type TorrentStatsDict = IndexMap<Shared<InfoHash>, TorrentStats, RandomState>;
