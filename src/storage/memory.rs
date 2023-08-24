use ahash::RandomState;
use async_trait::async_trait;
use indexmap::IndexMap;
use tokio::sync::RwLock;

use super::{Processor, Result, Storage};
use crate::{
    constants::TRACKER_ERROR_NOT_FOUND_TORRENT,
    models::{
        common::{InfoHash, IpType},
        peer::{Peer, PeerType},
        torrent::{
            PeerDict, PeerIdKey, SwarmStats, Torrent, TorrentStats, TorrentStatsList, TorrentSwarm,
        },
    },
};

static DEFAULT_SHARDS: usize = 1024;

#[derive(Debug)]
pub struct MemoryStorage {
    shards: Vec<Shard>,
}

#[derive(Debug, Default)]
struct Shard {
    torrents: RwLock<TorrentsMap>,
    swarms: RwLock<SwarmsMap>,
}

macro_rules! process_peers {
    ($swarm:expr, $stats:expr, $processor:expr, $( $peers_field:ident ),*) => {
        $(
            if !$processor.process(&$swarm.$peers_field) {
                return Ok($stats);
            }
        )*
    };
}

macro_rules! write_swarm {
    ($self:expr, $info_hash:expr, $ip_type:expr) => {
        $self
            .get_shard($info_hash)
            .swarms
            .write()
            .await
            .get_mut_or_insert_swarm($info_hash, $ip_type)?
    };
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn insert_torrent(&self, info_hash: &InfoHash, torrent: Option<Torrent>) -> Result<()> {
        let shard = self.get_shard(&info_hash);

        shard
            .torrents
            .write()
            .await
            .insert(info_hash.clone(), torrent.unwrap_or_default());

        Ok(())
    }

    async fn has_torrent(&self, info_hash: &InfoHash) -> Result<bool> {
        Ok(self
            .get_shard(&info_hash)
            .torrents
            .read()
            .await
            .contains_key(info_hash))
    }

    async fn remove_torrent(&mut self, info_hash: &InfoHash) -> Result<()> {
        let shard = self.get_shard(&info_hash);
        shard.swarms.write().await.remove(info_hash);
        shard.torrents.write().await.remove(info_hash);

        Ok(())
    }

    async fn get_torrent_stats(
        &self,
        info_hash: &InfoHash,
        ip_type: IpType,
    ) -> Result<TorrentStats> {
        let shard = self.get_shard(&info_hash);

        let completed = shard
            .torrents
            .read()
            .await
            .get_torrent(info_hash)
            .map(|tor| tor.completed)?;

        let (seeders, incomplete) = shard
            .swarms
            .read()
            .await
            .get_swarm(info_hash, ip_type)
            .map(|s| (s.complete_count(), s.incomplete_count()))
            .unwrap_or_default();

        Ok(TorrentStats {
            completed,
            seeders,
            incomplete,
        })
    }

    async fn get_multi_torrent_stats(
        &self,
        info_hashes: Vec<InfoHash>,
        ip_type: IpType,
    ) -> Result<Vec<(InfoHash, TorrentStats)>> {
        let mut result = Vec::with_capacity(info_hashes.len());
        for info_hash in info_hashes {
            if let Ok(stat) = self.get_torrent_stats(&info_hash, ip_type).await {
                result.push((info_hash, stat));
            }
        }

        Ok(result)
    }

    async fn get_all_torrent_stats(
        &self,
        processor: &mut dyn Processor<TorrentStatsList>,
    ) -> Result<()> {
        for shard in &self.shards {
            let mut stats = {
                let torrents = shard.torrents.read().await;
                let mut stats = Vec::with_capacity(torrents.len());

                for (info_hash, torrent) in torrents.iter() {
                    stats.push((
                        info_hash.clone(),
                        TorrentStats::new_with_completed(torrent.completed),
                    ));
                }

                stats
            };

            let swarms = shard.swarms.read().await;
            for (info_hash, swarm) in stats.iter_mut() {
                if let Some(s) = swarms.get(info_hash, IpType::V4) {
                    swarm.seeders = s.complete_count();
                    swarm.incomplete = s.incomplete_count();
                }

                if let Some(s) = swarms.get(info_hash, IpType::V6) {
                    swarm.seeders += s.complete_count();
                    swarm.incomplete += s.incomplete_count();
                }
            }

            if !processor.process(&stats) {
                return Ok(());
            }
        }

        Ok(())
    }

    async fn put_peer_in_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id_key: &PeerIdKey,
        peer: Peer,
        peer_type: PeerType,
    ) -> Result<()> {
        write_swarm!(self, info_hash, peer.ip_type()).insert_peer(
            peer_id_key.clone(),
            peer,
            peer_type,
        );

        Ok(())
    }

    async fn promote_peer_in_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id_key: &PeerIdKey,
        peer: Peer,
    ) -> Result<()> {
        if write_swarm!(self, info_hash, peer.ip_type()).promote_peer(peer_id_key, peer) {
            self.get_shard(info_hash)
                .torrents
                .write()
                .await
                .get_mut_torrent(info_hash)?
                .incr_completed();
        }

        Ok(())
    }

    async fn update_or_put_peer_in_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id_key: &PeerIdKey,
        peer: Peer,
        peer_type: PeerType,
    ) -> Result<()> {
        write_swarm!(self, info_hash, peer.ip_type()).update_or_insert_peer(
            peer_id_key,
            peer,
            peer_type,
        );

        Ok(())
    }

    async fn extract_peers_from_swarm(
        &self,
        info_hash: &InfoHash,
        peer_type: PeerType,
        peer_ip_type: IpType,
        processor: &mut dyn Processor<PeerDict>,
    ) -> Result<SwarmStats> {
        let swarms = self.get_shard(info_hash).swarms.read().await;
        let swarm = swarms.get_swarm(info_hash, peer_ip_type)?;

        let stats = SwarmStats {
            complete: swarm.complete_count(),
            incomplete: swarm.incomplete_count(),
        };

        match peer_type {
            PeerType::Leecher => {
                process_peers!(swarm, stats, processor, seeders, leechers, partial_seeds);
            }
            _ => process_peers!(swarm, stats, processor, leechers),
        };

        Ok(stats)
    }

    async fn remove_peer_from_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id: &PeerIdKey,
        peer_type: PeerType,
        peer_ip_type: IpType,
    ) -> Result<()> {
        let shard = self.get_shard(info_hash);
        let mut swarm_map = shard.swarms.write().await;

        if let Some(s) = swarm_map.get_mut(info_hash, peer_ip_type) {
            s.remove_peer(peer_id, peer_type);
        }

        Ok(())
    }
}

impl MemoryStorage {
    #[must_use]
    pub fn new() -> Self {
        Self::with_shards(DEFAULT_SHARDS)
    }

    #[must_use]
    pub fn with_shards(shard_count: usize) -> Self {
        assert!(shard_count > 0);

        let mut shards = Vec::with_capacity(shard_count);
        for _i in 0..shard_count {
            shards.push(Shard::default());
        }

        Self { shards }
    }

    fn get_shard(&self, info_hash: &InfoHash) -> &Shard {
        &self.shards[self.get_shard_index(info_hash.as_ref())]
    }

    fn get_shard_index(&self, data: &[u8]) -> usize {
        let bytes = data[0..4].try_into().unwrap();
        u32::from_be_bytes(bytes) as usize % self.shards.len()
    }
}

type TorrentsMap = IndexMap<InfoHash, Torrent, RandomState>;
type TorrentSwarmDict = IndexMap<InfoHash, TorrentSwarm, RandomState>;

#[derive(Debug, Default)]
struct SwarmsMap {
    /// The IPv4 swarm.
    v4: TorrentSwarmDict,
    /// The IPv6 swarm.
    v6: TorrentSwarmDict,
}

impl SwarmsMap {
    fn get_index_of(&self, info_hash: &InfoHash, ip_type: IpType) -> Option<usize> {
        match ip_type {
            IpType::V4 => self.v4.get_index_of(info_hash),
            IpType::V6 => self.v6.get_index_of(info_hash),
        }
    }

    fn get_index_mut(&mut self, index: usize, ip_type: IpType) -> Option<&mut TorrentSwarm> {
        match ip_type {
            IpType::V4 => self.v4.get_index_mut(index).map(|(_, v)| v),
            IpType::V6 => self.v6.get_index_mut(index).map(|(_, v)| v),
        }
    }

    fn get(&self, info_hash: &InfoHash, ip_type: IpType) -> Option<&TorrentSwarm> {
        match ip_type {
            IpType::V4 => self.v4.get(info_hash),
            IpType::V6 => self.v6.get(info_hash),
        }
    }

    fn get_mut(&mut self, info_hash: &InfoHash, ip_type: IpType) -> Option<&mut TorrentSwarm> {
        match ip_type {
            IpType::V4 => self.v4.get_mut(info_hash),
            IpType::V6 => self.v6.get_mut(info_hash),
        }
    }

    fn remove(&mut self, info_hash: &InfoHash) {
        self.v4.remove(info_hash);
        self.v6.remove(info_hash);
    }
}

trait SwarmGet {
    fn get_swarm(&self, info_hash: &InfoHash, ip_type: IpType) -> Result<&TorrentSwarm>;
}

impl SwarmGet for tokio::sync::RwLockReadGuard<'_, SwarmsMap> {
    fn get_swarm(&self, info_hash: &InfoHash, ip_type: IpType) -> Result<&TorrentSwarm> {
        match self.get(info_hash, ip_type) {
            Some(torrent) => Ok(torrent),
            None => Err(TRACKER_ERROR_NOT_FOUND_TORRENT.into()),
        }
    }
}

trait SwarmGetMut {
    fn get_mut_or_insert_swarm(
        &mut self,
        info_hash: &InfoHash,
        ip_type: IpType,
    ) -> Result<&mut TorrentSwarm>;

    fn get_mut_swarm(&mut self, info_hash: &InfoHash, ip_type: IpType)
        -> Result<&mut TorrentSwarm>;
}

impl SwarmGetMut for tokio::sync::RwLockWriteGuard<'_, SwarmsMap> {
    fn get_mut_swarm(
        &mut self,
        info_hash: &InfoHash,
        ip_type: IpType,
    ) -> Result<&mut TorrentSwarm> {
        match self.get_mut(info_hash, ip_type) {
            Some(torrent) => Ok(torrent),
            None => Err(TRACKER_ERROR_NOT_FOUND_TORRENT.into()),
        }
    }

    fn get_mut_or_insert_swarm(
        &mut self,
        info_hash: &InfoHash,
        ip_type: IpType,
    ) -> Result<&mut TorrentSwarm> {
        if let Some(index) = self.get_index_of(info_hash, ip_type) {
            return match self.get_index_mut(index, ip_type) {
                Some(torrent) => Ok(torrent),
                None => Err(TRACKER_ERROR_NOT_FOUND_TORRENT.into()),
            };
        }

        Ok(match ip_type {
            IpType::V4 => self.v4.entry(info_hash.clone()).or_default(),
            IpType::V6 => self.v6.entry(info_hash.clone()).or_default(),
        })
    }
}

trait TorrentGetMut {
    fn get_mut_torrent(&mut self, info_hash: &InfoHash) -> Result<&mut Torrent>;
}

impl TorrentGetMut for tokio::sync::RwLockWriteGuard<'_, TorrentsMap> {
    fn get_mut_torrent(&mut self, info_hash: &InfoHash) -> Result<&mut Torrent> {
        match self.get_mut(info_hash) {
            Some(torrent) => Ok(torrent),
            None => Err(TRACKER_ERROR_NOT_FOUND_TORRENT.into()),
        }
    }
}

trait TorrentGet {
    fn get_torrent(&mut self, info_hash: &InfoHash) -> Result<&Torrent>;
}

impl TorrentGet for tokio::sync::RwLockReadGuard<'_, TorrentsMap> {
    fn get_torrent(&mut self, info_hash: &InfoHash) -> Result<&Torrent> {
        match self.get(info_hash) {
            Some(torrent) => Ok(torrent),
            None => Err(TRACKER_ERROR_NOT_FOUND_TORRENT.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use ts_utils::time::Clock;

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
        let torrent = Default::default();

        storage
            .insert_torrent(&info_hash, Some(torrent))
            .await
            .unwrap();

        storage
    }

    fn create_test_peer() -> (PeerIdKey, Peer) {
        let peer_id: PeerId = PEER_ID.as_bytes().try_into().unwrap();
        let peer_id_key: PeerIdKey = PeerIdKey::new(&peer_id, None);

        let addr: PeerAddr = (Ipv4Addr::from([127, 0, 0, 1]), Port(8080)).into();
        let peer = Peer {
            addr,
            expire_at: Clock::now_since_epoch(),
        };

        (peer_id_key, peer)
    }

    #[tokio::test]
    async fn test_insert_torrent() {
        let storage = create_storage().await;
        let info_hash: InfoHash = INFOHASH_A.parse().unwrap();

        assert!(storage
            .get_shard(&info_hash)
            .torrents
            .read()
            .await
            .contains_key(&info_hash));
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

        assert!(!storage
            .get_shard(&info_hash)
            .torrents
            .read()
            .await
            .contains_key(&info_hash));
    }

    #[tokio::test]
    async fn test_get_torrent_stats() {
        let storage = create_storage().await;
        let info_hash: InfoHash = INFOHASH_A.parse().unwrap();
        let stats = storage.get_torrent_stats(&info_hash, IpType::V4).await;

        assert!(stats.is_ok());
    }

    #[tokio::test]
    async fn test_get_torrent_stats_not_found() {
        let storage = create_storage().await;
        let info_hash: InfoHash = INFOHASH_B.parse().unwrap();
        let stats = storage.get_torrent_stats(&info_hash, IpType::V4).await;

        assert!(stats.is_err());
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

        let swarms = storage.get_shard(&info_hash).swarms.read().await;
        let swarm = swarms.get_swarm(&info_hash, IpType::V4).unwrap();

        assert!(swarm.leechers.contains_key(&peer_id_key));
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
            let swarms = storage.get_shard(&info_hash).swarms.read().await;
            let swarm = swarms.get_swarm(&info_hash, IpType::V4).unwrap();
            assert!(swarm.leechers.contains_key(&peer_id_key));
        }

        storage
            .promote_peer_in_swarm(&info_hash, &peer_id_key, peer)
            .await
            .unwrap();

        {
            let swarms = storage.get_shard(&info_hash).swarms.read().await;
            let swarm = swarms.get_swarm(&info_hash, IpType::V4).unwrap();
            assert!(swarm.seeders.contains_key(&peer_id_key));
        }
    }
}
