use crate::config::StorageType;
use crate::models::common::InfoHash;
use crate::models::peer::{Peer, PeerType};
use crate::models::torrent::{PeerDict, PeerIdKey, TorrentStats, TorrentStatsDict};
use async_trait::async_trait;

mod memory;
pub use self::memory::MemoryStorage;

#[cfg(feature = "redis-store")]
mod redis;
pub use self::redis::RedisStorage;

#[async_trait]
pub trait Storage: Sync + Send {
    async fn insert_torrent(
        &self,
        info_hash: &InfoHash,
        torrent_stats: Option<TorrentStats>,
    ) -> Result<()>;

    async fn remove_torrent(&mut self, info_hash: &InfoHash) -> Result<()>;
    async fn has_torrent(&self, info_hash: &InfoHash) -> Result<bool>;

    async fn get_torrent_stats(&self, info_hash: &InfoHash) -> Result<Option<TorrentStats>>;

    async fn get_multi_torrent_stats(
        &self,
        info_hashes: Vec<InfoHash>,
    ) -> Result<Vec<(InfoHash, TorrentStats)>>;

    async fn get_all_torrent_stats(
        &self,
        processor: &mut dyn Processor<TorrentStatsDict>,
    ) -> Result<()>;

    async fn put_peer_in_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id: &PeerIdKey,
        peer: Peer,
        peer_type: PeerType,
    ) -> Result<()>;

    async fn update_or_put_peer_in_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id: &PeerIdKey,
        peer: Peer,
        peer_type: PeerType,
    ) -> Result<()>;

    async fn promote_peer_in_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id: &PeerIdKey,
        peer: Peer,
    ) -> Result<()>;

    async fn extract_peers_from_swarm(
        &self,
        info_hash: &InfoHash,
        peer_type: PeerType,
        processor: &mut dyn Processor<PeerDict>,
    ) -> Result<()>;

    async fn remove_peer_from_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id: &PeerIdKey,
        peer_type: PeerType,
    ) -> Result<Option<Peer>>;
}

pub type Error = String;
pub type Result<T> = std::result::Result<T, Error>;

pub fn create_new_storage(config: &crate::config::TSConfig) -> Result<Box<dyn Storage>> {
    let storage_type = config.storage.name.to_owned();

    match storage_type {
        StorageType::Memory => Ok(Box::new(MemoryStorage::new())),
        #[cfg(feature = "redis-store")]
        StorageType::Redis => {
            let redis_config = config.storage.redis.clone().unwrap();
            Ok(Box::new(RedisStorage::new(redis_config)))
        }
    }
}

pub trait Processor<Input>: Send {
    /// Process the input data and return a boolean value indicating whether
    /// processing should continue or stop.
    ///
    /// # Arguments
    ///
    /// - `input`: The input data to be processed.
    ///
    /// # Returns
    ///
    /// A boolean value indicating whether processing should continue (true) or stop (false).
    fn process(&mut self, input: &Input) -> bool;
}
