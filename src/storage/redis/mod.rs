mod args;
mod loader;
mod manager;

use async_trait::async_trait;
use log::debug;
use redis::{cmd, AsyncCommands, RedisResult, Script};
use std::{
    ops::DerefMut,
    sync::Arc,
    time::{self, Duration},
};
use ts_cache::{Cache, Policy};
use ts_pool::{Pool, PooledConnection};

use self::{
    args::{SwarmKey, TorrentKey, TORRENT_COMPLETED_KEY},
    loader::{SwarmStatsKey, SwarmStatsLoader, TorrentLoader},
    manager::{get_connection, RedisConnectionManager},
};
use super::{PeerExtractor, Processor, Result, Storage};
use crate::{
    config::TSConfig,
    models::{
        common::{InfoHash, IpType},
        peer::{Peer, PeerType},
        torrent::{PeerIdKey, SwarmStats, Torrent, TorrentStats, TorrentStatsList},
    },
};

#[derive(Debug)]
pub struct RedisStorage {
    pool: Arc<Pool<RedisConnectionManager>>,
    peer_idle_time_secs: usize,
    torrent_cache: Cache<InfoHash, Torrent>,
    swarm_stats_cache: Cache<SwarmStatsKey, SwarmStats>,
}

impl RedisStorage {
    pub fn new(config: Arc<TSConfig>) -> Self {
        let redis_config = config
            .storage
            .redis
            .as_ref()
            .expect("Redis config is not set");

        let manager = RedisConnectionManager::new(redis_config.url.clone());

        let pool = Pool::builder()
            .max_size(redis_config.max_connections)
            .min_idle(redis_config.min_idle_connections)
            .idle_timeout(redis_config.idle_connection_time)
            .reaper_rate(redis_config.idle_connection_time + time::Duration::from_secs(1))
            .connection_timeout(redis_config.max_connection_wait_time)
            .test_on_check_out(false)
            .build(manager)
            .expect("Failed to create redis pool");

        let pool = Arc::new(pool);
        let peer_idle_time_secs = config.tracker.peer_idle_time.as_secs() as usize;

        let torrent_cache = Cache::<InfoHash, Torrent>::builder()
            .expiry(Duration::from_secs(5))
            .policy(Policy::RefreshBeforeAccess)
            .build(TorrentLoader::new(pool.clone()));

        let swarm_stats_cache = Cache::<InfoHash, SwarmStats>::builder()
            .expiry(Duration::from_secs(15))
            .policy(Policy::RefreshAfterAccess)
            .build(SwarmStatsLoader::new(pool.clone()));

        Self {
            pool,
            peer_idle_time_secs,
            torrent_cache,
            swarm_stats_cache,
        }
    }

    pub async fn get_connection(&self) -> Result<PooledConnection<'_, RedisConnectionManager>> {
        get_connection(&self.pool).await
    }
}

#[async_trait]
impl Storage for RedisStorage {
    async fn insert_torrent(&self, info_hash: &InfoHash, torrent: Option<Torrent>) -> Result<()> {
        let _ = cmd("HSETNX")
            .arg(TorrentKey(info_hash))
            .arg(TORRENT_COMPLETED_KEY)
            .arg(torrent.map(|v| v.completed).unwrap_or_default())
            .query_async(self.get_connection().await?.deref_mut())
            .await?;

        Ok(())
    }

    async fn get_torrent(&self, info_hash: &InfoHash) -> Result<Option<Torrent>> {
        Ok(self.torrent_cache.get(info_hash.into()).await)
    }

    async fn has_torrent(&self, info_hash: &InfoHash) -> Result<bool> {
        Ok(self.torrent_cache.get(info_hash.into()).await.is_some())
    }

    async fn remove_torrent(&mut self, info_hash: &InfoHash) -> Result<()> {
        self.get_connection()
            .await?
            .del(TorrentKey(info_hash))
            .await?;

        let _ = self.torrent_cache.invalidate(info_hash.into()).await;

        Ok(())
    }

    async fn get_torrent_stats(
        &self,
        info_hash: &InfoHash,
        ip_type: IpType,
    ) -> Result<TorrentStats> {
        let (swarm_stats, torrent) = tokio::join!(
            self.swarm_stats_cache
                .get((info_hash.clone(), ip_type).into()),
            self.torrent_cache.get(info_hash.into())
        );

        let swarm_stats = swarm_stats.unwrap_or_default();
        let torrent = torrent.unwrap_or_default();

        Ok(TorrentStats {
            seeders: swarm_stats.complete,
            completed: torrent.completed,
            incomplete: swarm_stats.incomplete,
        })
    }

    async fn get_multi_torrent_stats(
        &self,
        info_hashes: Vec<InfoHash>,
        ip_type: IpType,
    ) -> Result<TorrentStatsList> {
        let mut pipe = redis::pipe();

        for info_hash in &info_hashes {
            let torrent_key = TorrentKey(info_hash).encode();
            let (swarm_key_leecher, swarm_key_seeder, swarm_key_partial) =
                SwarmKey::get_all_swarm_keys(torrent_key.as_ref(), ip_type);

            pipe.hget(torrent_key.as_ref(), TORRENT_COMPLETED_KEY)
                .hlen(swarm_key_leecher)
                .hlen(swarm_key_seeder)
                .hlen(swarm_key_partial);
        }

        let mut conn = self.get_connection().await?;
        let results: Vec<TorrentStatsTuple> = pipe.query_async(conn.deref_mut()).await?;

        let results = info_hashes.into_iter().zip(results.into_iter().map(|v| {
            let (completed, leechers, seeders, partial_seeds) = v;
            let incomplete = leechers + partial_seeds;
            let completed = completed.unwrap_or_default();

            TorrentStats {
                completed,
                seeders,
                incomplete,
            }
        }));

        Ok(results.collect::<Vec<_>>())
    }

    async fn get_all_torrent_stats(
        &self,
        _processor: &mut dyn Processor<TorrentStatsList>,
    ) -> Result<()> {
        debug!("get_all_torrent_stats is not implemented for redis storage");
        Ok(())
    }

    async fn put_peer_in_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id_key: &PeerIdKey,
        peer: Peer,
        peer_type: PeerType,
    ) -> Result<()> {
        self.update_or_put_peer_in_swarm(info_hash, peer_id_key, peer, peer_type)
            .await
    }

    async fn promote_peer_in_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id_key: &PeerIdKey,
        peer: Peer,
    ) -> Result<()> {
        let torrent_key = TorrentKey(info_hash).encode();
        let old_swarm_key = SwarmKey {
            torrent_key: torrent_key.as_ref(),
            peer_type: PeerType::Leecher,
            peer_ip_type: peer.ip_type(),
        };

        let new_swarm_key = SwarmKey {
            torrent_key: torrent_key.as_ref(),
            peer_type: PeerType::Seeder,
            peer_ip_type: peer.ip_type(),
        };

        let mut insert_peer = cmd("HMSET");
        insert_peer
            .arg(&new_swarm_key)
            .arg(peer_id_key.as_ref())
            .arg(peer);

        let mut conn = self.get_connection().await?;
        let (is_completed,): (bool,) = redis::pipe()
            .hdel(old_swarm_key, peer_id_key.as_ref())
            .add_command(insert_peer)
            .ignore()
            .expire(&new_swarm_key, self.peer_idle_time_secs)
            .ignore()
            .query_async(conn.deref_mut())
            .await?;

        if is_completed {
            let _: RedisResult<usize> = cmd("HINCRBY")
                .arg(torrent_key.as_ref())
                .arg(TORRENT_COMPLETED_KEY)
                .arg(1)
                .query_async(conn.deref_mut())
                .await;
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
        let torrent_key = TorrentKey(info_hash).encode();
        let swarm_key = SwarmKey {
            peer_type,
            torrent_key: torrent_key.as_ref(),
            peer_ip_type: peer.ip_type(),
        };

        let mut insert_peer = cmd("HMSET");
        insert_peer
            .arg(&swarm_key)
            .arg(peer_id_key.as_ref())
            .arg(peer);

        redis::pipe()
            .add_command(insert_peer)
            .expire(&swarm_key, self.peer_idle_time_secs)
            .ignore()
            .query_async(self.get_connection().await?.deref_mut())
            .await?;

        Ok(())
    }

    async fn extract_peers_from_swarm(
        &self,
        info_hash: &InfoHash,
        peer_type: PeerType,
        ip_type: IpType,
        _extractor: &mut dyn PeerExtractor,
    ) -> Result<SwarmStats> {
        let stats = self
            .swarm_stats_cache
            .get((info_hash.clone(), ip_type).into())
            .await
            .unwrap_or_default();

        match peer_type {
            PeerType::Leecher => {}
            _ => {}
        }

        Ok(stats)
    }

    async fn remove_peer_from_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id_key: &PeerIdKey,
        peer_type: PeerType,
        peer_ip_type: IpType,
    ) -> Result<()> {
        let torrent_key = TorrentKey(info_hash).encode();
        let swarm_key = SwarmKey {
            torrent_key: torrent_key.as_ref(),
            peer_type,
            peer_ip_type,
        };

        let mut conn = self.get_connection().await?;
        cmd("hdel")
            .arg(swarm_key)
            .arg(peer_id_key.as_ref())
            .query_async(conn.deref_mut())
            .await?;

        Ok(())
    }
}

/// The tuple of torrent stats. (completed, leechers, seeders, partial seeds)
type TorrentStatsTuple = (Option<u32>, u32, u32, u32);

// let result = SCRIPT
//     .key(info_hash)
//     .arg(1)
//     .arg(2)
//     .invoke_async(conn.deref_mut())
//     .await;

// assert_eq!(result, Ok(3));

lazy_static! {
    static ref SCRIPT: Script = redis::Script::new(
        r"
            return tonumber(ARGV[1]) + tonumber(ARGV[2]);
        "
    );
}
