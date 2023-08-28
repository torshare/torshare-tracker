use super::args::SwarmKey;
use super::manager::get_connection;
use super::Result;
use super::{args::TorrentKey, manager::RedisConnectionManager};
use crate::models::common::IpType;
use crate::models::torrent::SwarmStats;
use crate::models::{common::InfoHash, torrent::Torrent};
use async_trait::async_trait;
use log::error;
use redis::cmd;
use std::ops::DerefMut;
use std::slice;
use std::sync::Arc;
use ts_cache::CacheLoader;
use ts_pool::Pool;

pub struct TorrentLoader {
    pool: Arc<Pool<RedisConnectionManager>>,
}

impl TorrentLoader {
    async fn get_torrent(&self, info_hash: &InfoHash) -> Result<Option<Torrent>> {
        let torrent: Option<Torrent> = cmd("HGETALL")
            .arg(TorrentKey(info_hash))
            .query_async(get_connection(&self.pool).await?.deref_mut())
            .await?;

        Ok(torrent)
    }

    async fn get_multiple_torrents(&self, info_hash: &[InfoHash]) -> Result<Vec<Torrent>> {
        let mut pipe = redis::pipe();
        for info_hash in info_hash {
            pipe.hgetall(TorrentKey(info_hash));
        }

        let torrents: Vec<Torrent> = pipe
            .query_async(get_connection(&self.pool).await?.deref_mut())
            .await?;

        Ok(torrents)
    }

    pub fn new(
        pool: Arc<Pool<RedisConnectionManager>>,
    ) -> Arc<dyn CacheLoader<Key = InfoHash, Value = Torrent>> {
        Arc::new(Self { pool })
    }
}

#[async_trait]
impl CacheLoader for TorrentLoader {
    type Key = InfoHash;
    type Value = Torrent;

    async fn load(&self, key: &Self::Key) -> Option<Self::Value> {
        match self.get_torrent(key).await {
            Ok(torrent) => torrent,
            Err(err) => {
                error!("Failed to load torrent from redis: {:?}", err);
                None
            }
        }
    }

    async fn load_all(&self, keys: &[Self::Key]) -> Vec<(Self::Key, Self::Value)> {
        match self.get_multiple_torrents(keys).await {
            Ok(torrents) => keys
                .iter()
                .zip(torrents.into_iter())
                .map(|(key, value)| (key.clone(), value))
                .collect(),
            Err(err) => {
                error!("Failed to load torrents from redis: {:?}", err);
                vec![]
            }
        }
    }
}

pub struct SwarmStatsLoader {
    pool: Arc<Pool<RedisConnectionManager>>,
}

impl SwarmStatsLoader {
    pub fn new(
        pool: Arc<Pool<RedisConnectionManager>>,
    ) -> Arc<dyn CacheLoader<Key = SwarmStatsKey, Value = SwarmStats>> {
        Arc::new(Self { pool })
    }

    async fn get_swarm_stats(&self, info_hash: &InfoHash, ip_type: IpType) -> Result<SwarmStats> {
        let results = self
            .get_multiple_swarm_stats(slice::from_ref(&info_hash), ip_type)
            .await?;

        Ok(results.into_iter().next().unwrap_or_default())
    }

    async fn get_multiple_swarm_stats(
        &self,
        info_hash: &[&InfoHash],
        ip_type: IpType,
    ) -> Result<Vec<SwarmStats>> {
        let mut pipe = redis::pipe();

        for info_hash in info_hash {
            let torrent_key = TorrentKey(info_hash).encode();
            let (swarm_key_leecher, swarm_key_seeder, swarm_key_partial) =
                SwarmKey::get_all_swarm_keys(torrent_key.as_ref(), ip_type);

            pipe.hlen(swarm_key_leecher)
                .hlen(swarm_key_seeder)
                .hlen(swarm_key_partial);
        }

        let results: Vec<(u32, u32, u32)> = pipe
            .query_async(get_connection(&self.pool).await?.deref_mut())
            .await?;

        let results = results
            .into_iter()
            .map(|(l, s, p)| SwarmStats {
                incomplete: l + p,
                complete: s,
            })
            .collect();

        Ok(results)
    }
}

pub type SwarmStatsKey = (InfoHash, IpType);

macro_rules! collect_swarm_stats {
    ($self:expr, $keys:expr, $results:expr, $ip_type:expr) => {
        let info_hashes = $keys
            .iter()
            .filter(|(_, ip_type)| *ip_type == $ip_type)
            .map(|(info_hash, _)| info_hash)
            .collect::<Vec<&InfoHash>>();

        if !info_hashes.is_empty() {
            match $self.get_multiple_swarm_stats(info_hashes.as_slice(), $ip_type).await {
                Ok(stats) => {
                    $results.extend(
                        info_hashes
                            .into_iter()
                            .zip(stats.into_iter())
                            .map(|(info_hash, stats)| ((info_hash.clone(), $ip_type), stats)),
                    );
                }
                Err(err) => {
                    error!("Failed to load swarm stats from redis: {:?}", err);
                }
            }
        }
    };
}

#[async_trait]
impl CacheLoader for SwarmStatsLoader {
    type Key = SwarmStatsKey;
    type Value = SwarmStats;

    async fn load(&self, (info_hash, ip_type): &Self::Key) -> Option<Self::Value> {
        match self.get_swarm_stats(info_hash, ip_type.to_owned()).await {
            Ok(stats) => Some(stats),
            Err(err) => {
                error!("Failed to load swarm stats from redis: {:?}", err);
                None
            }
        }
    }

    async fn load_all(&self, keys: &[Self::Key]) -> Vec<(Self::Key, Self::Value)> {
        let mut results = Vec::with_capacity(keys.len());
        collect_swarm_stats!(self, keys, results, IpType::V4);
        collect_swarm_stats!(self, keys, results, IpType::V6);
        results
    }
}
