use async_trait::async_trait;
use tokio::sync::RwLock;

use super::{Processor, Result, Storage};
use crate::{
    constants::TRACKER_ERROR_NOT_FOUND_TORRENT,
    models::{
        common::InfoHash,
        peer::{Peer, PeerType},
        torrent::{
            PeerDict, PeerIdKey, TorrentStats, TorrentStatsDict, TorrentSwarm, TorrentSwarmDict,
        },
    },
};

#[derive(Debug)]
pub struct MemoryStorage {
    swarms: RwLock<TorrentSwarmDict>,
    stats: RwLock<TorrentStatsDict>,
}

impl MemoryStorage {
    #[must_use]
    pub fn new() -> Self {
        Self {
            swarms: RwLock::new(Default::default()),
            stats: RwLock::new(Default::default()),
        }
    }
}

macro_rules! process_peers {
    ($swarm:expr, $processor:expr, $( $peers_method:ident ),*) => {
        $(
            if !$processor.process($swarm.$peers_method()) {
                return Ok(());
            }
        )*
    };
}

macro_rules! write_swarm {
    ($self:expr, $info_hash:expr) => {
        $self.swarms.write().await.get_mut_swarm($info_hash)?
    };
}

macro_rules! write_stats {
    ($self:expr, $info_hash:expr) => {
        $self.stats.write().await.get_mut_stats($info_hash)?
    };
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn insert_torrent(
        &self,
        info_hash: &InfoHash,
        tor_stats: Option<TorrentStats>,
    ) -> Result<()> {
        self.swarms
            .write()
            .await
            .insert(info_hash.clone(), TorrentSwarm::default());

        self.stats.write().await.insert(
            info_hash.clone(),
            tor_stats.unwrap_or_else(|| TorrentStats::default()),
        );

        Ok(())
    }

    async fn has_torrent(&self, info_hash: &InfoHash) -> Result<bool> {
        Ok(self.stats.read().await.contains_key(info_hash))
    }

    async fn remove_torrent(&mut self, info_hash: &InfoHash) -> Result<()> {
        self.swarms.write().await.remove(info_hash);
        self.stats.write().await.remove(info_hash);

        Ok(())
    }

    async fn get_torrent_stats(&self, info_hash: &InfoHash) -> Result<Option<TorrentStats>> {
        Ok(self.stats.read().await.get(info_hash).cloned())
    }

    async fn get_multi_torrent_stats(
        &self,
        info_hashes: Vec<InfoHash>,
    ) -> Result<Vec<(InfoHash, TorrentStats)>> {
        let stats = self.stats.read().await;
        let mut result = Vec::with_capacity(info_hashes.len());

        for info_hash in info_hashes {
            if let Some(stat) = stats.get(&info_hash) {
                result.push((info_hash, stat.clone()));
            }
        }

        Ok(result)
    }

    async fn get_all_torrent_stats(
        &self,
        processor: &mut dyn Processor<TorrentStatsDict>,
    ) -> Result<()> {
        let stats = self.stats.read().await;
        let _ = processor.process(&stats);
        Ok(())
    }

    async fn put_peer_in_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id: &PeerIdKey,
        peer: Peer,
        peer_type: PeerType,
    ) -> Result<()> {
        write_swarm!(self, info_hash).insert_peer(peer_id.clone(), peer, peer_type);
        Ok(())
    }

    async fn promote_peer_in_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id: &PeerIdKey,
        peer: Peer,
    ) -> Result<()> {
        write_swarm!(self, info_hash).promote_peer(peer_id, peer);
        write_stats!(self, info_hash).incr_completed();

        Ok(())
    }

    async fn update_or_put_peer_in_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id: &PeerIdKey,
        peer: Peer,
        peer_type: PeerType,
    ) -> Result<()> {
        write_swarm!(self, info_hash).update_or_insert_peer(peer_id, peer, peer_type);
        Ok(())
    }

    async fn extract_peers_from_swarm(
        &self,
        info_hash: &InfoHash,
        peer_type: PeerType,
        processor: &mut dyn Processor<PeerDict>,
    ) -> Result<()> {
        let torrents = self.swarms.read().await;
        let swarm = torrents.get_swarm(info_hash)?;

        match peer_type {
            PeerType::Leecher => {
                process_peers!(
                    swarm,
                    processor,
                    get_seeders,
                    get_leechers,
                    get_partial_seeds
                );
            }
            _ => process_peers!(swarm, processor, get_leechers),
        };

        Ok(())
    }

    async fn remove_peer_from_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id: &PeerIdKey,
        peer_type: PeerType,
    ) -> Result<Option<Peer>> {
        Ok(write_swarm!(self, info_hash).remove_peer(&peer_id, peer_type))
    }
}

trait SwarmGet {
    fn get_swarm(&self, info_hash: &InfoHash) -> Result<&TorrentSwarm>;
}

impl SwarmGet for tokio::sync::RwLockReadGuard<'_, TorrentSwarmDict> {
    fn get_swarm(&self, info_hash: &InfoHash) -> Result<&TorrentSwarm> {
        match self.get(info_hash) {
            Some(torrent) => Ok(torrent),
            None => Err(TRACKER_ERROR_NOT_FOUND_TORRENT.to_string()),
        }
    }
}

trait SwarmGetMut {
    fn get_mut_swarm(&mut self, info_hash: &InfoHash) -> Result<&mut TorrentSwarm>;
}

impl SwarmGetMut for tokio::sync::RwLockWriteGuard<'_, TorrentSwarmDict> {
    fn get_mut_swarm(&mut self, info_hash: &InfoHash) -> Result<&mut TorrentSwarm> {
        match self.get_mut(info_hash) {
            Some(torrent) => Ok(torrent),
            None => Err(TRACKER_ERROR_NOT_FOUND_TORRENT.to_string()),
        }
    }
}

trait StatsGetMut {
    fn get_mut_stats(&mut self, info_hash: &InfoHash) -> Result<&mut TorrentStats>;
}

impl StatsGetMut for tokio::sync::RwLockWriteGuard<'_, TorrentStatsDict> {
    fn get_mut_stats(&mut self, info_hash: &InfoHash) -> Result<&mut TorrentStats> {
        match self.get_mut(info_hash) {
            Some(stats) => Ok(stats),
            None => Err(TRACKER_ERROR_NOT_FOUND_TORRENT.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{
        common::{PeerId, Port},
        peer::PeerAddr,
    };
    use std::net::Ipv4Addr;

    use super::*;

    const INFOHASH_A: &str = "2a7b9e1f5c8d3a6b0f2e4c5a9b7d1e3a6c8b5d99";
    const INFOHASH_B: &str = "3b8c2d0e6f9a4b7c1d4e5f6a2b8c3d9e4f5a6b7c";
    const PEER_ID: &str = "01234567890123456789";

    async fn create_storage() -> MemoryStorage {
        let storage = MemoryStorage::new();
        let info_hash: InfoHash = INFOHASH_A.parse().unwrap();
        let tor_stats = TorrentStats::default();

        storage
            .insert_torrent(&info_hash, Some(tor_stats))
            .await
            .unwrap();

        storage
    }

    fn create_test_peer() -> (PeerIdKey, Peer) {
        let peer_id: PeerId = PEER_ID.as_bytes().try_into().unwrap();
        let peer_id_key: PeerIdKey = PeerIdKey::new(&peer_id, None);

        let addr_v4: PeerAddr = (Ipv4Addr::from([127, 0, 0, 1]), Port(8080)).into();

        let peer = Peer {
            addr_v4: Some(addr_v4),
            ..Default::default()
        };

        (peer_id_key, peer)
    }

    #[tokio::test]
    async fn test_insert_torrent() {
        let storage = create_storage().await;
        let info_hash: InfoHash = INFOHASH_A.parse().unwrap();

        assert!(storage.swarms.read().await.contains_key(&info_hash));
        assert!(storage.stats.read().await.contains_key(&info_hash));
    }

    #[tokio::test]
    async fn test_has_torrent() {
        let storage = create_storage().await;
        let info_hash: InfoHash = INFOHASH_A.parse().unwrap();
        assert!(storage.has_torrent(&info_hash).await.unwrap());
    }

    #[tokio::test]
    async fn test_remove_torrent() {
        let mut storage = create_storage().await;
        let info_hash: InfoHash = INFOHASH_A.parse().unwrap();

        storage.remove_torrent(&info_hash).await.unwrap();

        assert!(!storage.swarms.read().await.contains_key(&info_hash));
        assert!(!storage.stats.read().await.contains_key(&info_hash));
    }

    #[tokio::test]
    async fn test_get_torrent_stats() {
        let storage = create_storage().await;
        let info_hash: InfoHash = INFOHASH_A.parse().unwrap();

        let stats = storage.get_torrent_stats(&info_hash).await.unwrap();
        assert!(stats.is_some());
    }

    #[tokio::test]
    async fn test_get_torrent_stats_not_found() {
        let storage = create_storage().await;
        let info_hash: InfoHash = INFOHASH_B.parse().unwrap();

        let stats = storage.get_torrent_stats(&info_hash).await.unwrap();
        assert!(stats.is_none());
    }

    #[tokio::test]
    async fn test_get_torrent_swarm() {
        let storage = create_storage().await;

        let info_hash: InfoHash = INFOHASH_A.parse().unwrap();
        assert!(storage.swarms.read().await.get_swarm(&info_hash).is_ok());

        let info_hash: InfoHash = INFOHASH_B.parse().unwrap();
        assert!(storage.swarms.read().await.get_swarm(&info_hash).is_err());
    }

    #[tokio::test]
    async fn test_get_torrent_swarm_mut() {
        let storage = create_storage().await;

        let info_hash: InfoHash = INFOHASH_A.parse().unwrap();
        assert!(storage
            .swarms
            .write()
            .await
            .get_mut_swarm(&info_hash)
            .is_ok());

        let info_hash: InfoHash = INFOHASH_B.parse().unwrap();
        assert!(storage
            .swarms
            .write()
            .await
            .get_mut_swarm(&info_hash)
            .is_err());
    }

    #[tokio::test]
    async fn test_put_peer_in_swarm() {
        let storage = create_storage().await;
        let info_hash: InfoHash = INFOHASH_A.parse().unwrap();
        let (peer_id_key, peer) = create_test_peer();

        storage
            .put_peer_in_swarm(&info_hash, &peer_id_key, peer, PeerType::Leecher)
            .await
            .unwrap();

        let swarms = storage.swarms.read().await;
        let swarm = swarms.get_swarm(&info_hash).unwrap();

        assert!(swarm.get_leechers().contains_key(&peer_id_key));
    }

    #[tokio::test]
    async fn test_promote_peer_in_swarm() {
        let storage = create_storage().await;
        let info_hash: InfoHash = INFOHASH_A.parse().unwrap();
        let (peer_id_key, peer) = create_test_peer();

        storage
            .put_peer_in_swarm(&info_hash, &peer_id_key, peer.clone(), PeerType::Leecher)
            .await
            .unwrap();

        {
            let swarms = storage.swarms.read().await;
            let swarm = swarms.get_swarm(&info_hash).unwrap();
            assert!(swarm.get_leechers().contains_key(&peer_id_key));
        }

        storage
            .promote_peer_in_swarm(&info_hash, &peer_id_key, peer)
            .await
            .unwrap();

        {
            let swarms = storage.swarms.read().await;
            let swarm = swarms.get_swarm(&info_hash).unwrap();
            assert!(swarm.get_seeders().contains_key(&peer_id_key));
        }
    }
}
