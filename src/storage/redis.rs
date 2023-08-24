use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use redis::{
    cmd, AsyncCommands, FromRedisValue, IntoConnectionInfo, RedisResult, Script, ToRedisArgs, Value,
};
use std::{mem, ops::DerefMut, sync::Arc, time::Duration};
use ts_pool::{ManageConnection, Pool, PoolError, PooledConnection};

use super::{Processor, Result, Storage};
use crate::{
    config::TSConfig,
    models::{
        common::{InfoHash, IpType, INFOHASH_LENGTH},
        peer::{Peer, PeerType},
        torrent::{PeerDict, PeerIdKey, SwarmStats, Torrent, TorrentStats, TorrentStatsList},
    },
};

#[derive(Debug)]
#[allow(unused)]
pub struct RedisStorage {
    pool: Pool<RedisConnectionManager>,
    peer_idle_time_secs: usize,
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
            .idle_timeout(Duration::from_secs(10))
            .reaper_rate(Duration::from_secs(10))
            .test_on_check_out(false)
            .build(manager)
            .expect("Failed to create redis pool");

        let peer_idle_time_secs = config.tracker.peer_idle_time.as_secs() as usize;

        Self {
            pool,
            peer_idle_time_secs,
        }
    }

    pub async fn get_connection(&self) -> Result<PooledConnection<'_, RedisConnectionManager>> {
        match self.pool.get().await {
            Ok(Some(conn)) => Ok(conn),
            Ok(None) => Err("failed to get redis connection".into()),
            Err(err) => Err(err.into()),
        }
    }
}

#[async_trait]
impl Storage for RedisStorage {
    async fn insert_torrent(&self, info_hash: &InfoHash, torrent: Option<Torrent>) -> Result<()> {
        let _ = cmd("HMSET")
            .arg(TorrentKey(info_hash))
            .arg(torrent.unwrap_or_default())
            .query_async(self.get_connection().await?.deref_mut())
            .await?;

        Ok(())
    }

    async fn get_torrent(&self, info_hash: &InfoHash) -> Result<Option<Torrent>> {
        let torrent_key = TorrentKey(info_hash).encode();
        let torrent: Option<Torrent> = cmd("HGETALL")
            .arg(torrent_key.as_ref())
            .query_async(self.get_connection().await?.deref_mut())
            .await?;

        Ok(torrent)
    }

    async fn has_torrent(&self, info_hash: &InfoHash) -> Result<bool> {
        Ok(self
            .get_connection()
            .await?
            .exists(TorrentKey(info_hash))
            .await?)
    }

    async fn remove_torrent(&mut self, info_hash: &InfoHash) -> Result<()> {
        Ok(self
            .get_connection()
            .await?
            .del(TorrentKey(info_hash))
            .await?)
    }

    async fn get_torrent_stats(
        &self,
        info_hash: &InfoHash,
        ip_type: IpType,
    ) -> Result<TorrentStats> {
        let torrent_key = TorrentKey(info_hash).encode();
        let (swarm_key_leecher, swarm_key_seeder, swarm_key_partial) =
            SwarmKey::get_all_swarm_keys(torrent_key.as_ref(), ip_type);

        let (completed, leechers, seeders, partial_seeds): TorrentStatsTuple = redis::pipe()
            .hget(torrent_key.as_ref(), TORRENT_COMPLETED_KEY)
            .hlen(swarm_key_leecher)
            .hlen(swarm_key_seeder)
            .hlen(swarm_key_partial)
            .query_async(self.get_connection().await?.deref_mut())
            .await?;

        let incomplete = leechers + partial_seeds;
        let completed = completed.unwrap_or_default();

        let stats = TorrentStats {
            completed,
            seeders,
            incomplete,
        };

        Ok(stats)
    }

    async fn get_multi_torrent_stats(
        &self,
        info_hashes: Vec<InfoHash>,
        ip_type: IpType,
    ) -> Result<Vec<(InfoHash, TorrentStats)>> {
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
        _info_hash: &InfoHash,
        _peer_type: PeerType,
        _ip_type: IpType,
        _processor: &mut dyn Processor<PeerDict>,
    ) -> Result<SwarmStats> {
        Ok(SwarmStats::default())
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

impl From<redis::RedisError> for super::Error {
    fn from(err: redis::RedisError) -> Self {
        Self::runtime(Box::new(err))
    }
}

impl From<PoolError> for super::Error {
    fn from(err: PoolError) -> Self {
        Self::runtime(Box::new(err))
    }
}

struct SwarmKey<'a> {
    torrent_key: &'a [u8],
    peer_type: PeerType,
    peer_ip_type: IpType,
}

impl<'a> SwarmKey<'a> {
    fn encode(&self) -> Bytes {
        let mut bytes = BytesMut::with_capacity(SWARM_KEY_LEN);
        bytes.extend_from_slice(self.torrent_key);

        match self.peer_ip_type {
            IpType::V4 => bytes.extend_from_slice(TYPE_IPV4),
            IpType::V6 => bytes.extend_from_slice(TYPE_IPV6),
        }

        match self.peer_type {
            PeerType::Seeder => bytes.extend_from_slice(SWARM_KEY_SEEDER_PREFIX),
            PeerType::Leecher => bytes.extend_from_slice(SWARM_KEY_LEECHER_PREFIX),
            PeerType::Partial => bytes.extend_from_slice(SWARM_KEY_PARTIAL_SEED_PREFIX),
        }

        bytes.freeze()
    }

    fn get_all_swarm_keys(torrent_key: &'a [u8], ip_type: IpType) -> (Self, Self, Self) {
        let swark_key_leecher = SwarmKey {
            torrent_key,
            peer_type: PeerType::Leecher,
            peer_ip_type: ip_type,
        };

        let swark_key_seeder = SwarmKey {
            torrent_key,
            peer_type: PeerType::Seeder,
            peer_ip_type: ip_type,
        };

        let swark_key_partial = SwarmKey {
            torrent_key,
            peer_type: PeerType::Partial,
            peer_ip_type: ip_type,
        };

        (swark_key_leecher, swark_key_seeder, swark_key_partial)
    }
}

struct TorrentKey<'a>(&'a InfoHash);

impl<'a> TorrentKey<'a> {
    fn encode(&self) -> Bytes {
        let mut bytes = BytesMut::with_capacity(TORRENT_KEY_LEN);
        bytes.extend_from_slice(TORRENT_KEY_PREFIX);
        bytes.extend_from_slice(self.0.to_string().as_bytes());
        bytes.freeze()
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

const REDIS_KEY_PREFIX: &[u8] = b"ts_";

const TORRENT_KEY_PREFIX: &[u8] = REDIS_KEY_PREFIX;
const TORRENT_KEY_LEN: usize = TORRENT_KEY_PREFIX.len() + INFOHASH_LENGTH * 2;

const TYPE_IPV4: &[u8] = b"_v4";
const TYPE_IPV6: &[u8] = b"_v6";
const TYPE_LEN: usize = 3;

const SWARM_KEY_SEEDER_PREFIX: &[u8] = b"_s";
const SWARM_KEY_LEECHER_PREFIX: &[u8] = b"_l";
const SWARM_KEY_PARTIAL_SEED_PREFIX: &[u8] = b"_p";
const SWARM_KEY_LEN: usize = TORRENT_KEY_LEN + TYPE_LEN + 2;

const TORRENT_COMPLETED_KEY: &[u8] = b"c";

// Define the macro for serializing fields
macro_rules! serialize_field {
    ($out:ident, $field_name:expr, $field_value:expr) => {
        $field_name.write_redis_args($out);
        $field_value.write_redis_args($out);
    };
}

macro_rules! process_chunks_for_struct {
    ($chunks:expr, $struct:expr, $($pattern:expr => $field:ident),*) => {
        for item in $chunks {
            let field = &item[0];
            let value = &item[1];

            match field {
                Value::Data(ref field) => {
                    $(
                        if field.as_slice() == $pattern {
                            $struct.$field = FromRedisValue::from_redis_value(&value)?;
                        }
                    )*
                },
                _ => {}
            }
        }
    };
}

impl<'a> ToRedisArgs for TorrentKey<'a> {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        out.write_arg(&self.encode());
    }
}

impl<'a> ToRedisArgs for SwarmKey<'a> {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        out.write_arg(&self.encode());
    }
}

impl ToRedisArgs for Peer {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        let addr = self.addr.as_bytes();
        let expire_at = self.expire_at.as_secs();

        let len = addr.len() + mem::size_of::<u64>();
        let mut bytes = BytesMut::with_capacity(len);

        bytes.extend_from_slice(self.addr.as_bytes());
        bytes.extend_from_slice(&expire_at.to_be_bytes());

        out.write_arg(&bytes);
    }

    fn is_single_arg(&self) -> bool {
        true
    }
}

impl FromRedisValue for Peer {
    fn from_redis_value(_v: &Value) -> RedisResult<Self> {
        todo!()
    }
}

impl ToRedisArgs for Torrent {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        serialize_field!(out, TORRENT_COMPLETED_KEY, self.completed);
    }

    fn is_single_arg(&self) -> bool {
        false
    }
}

impl FromRedisValue for Torrent {
    fn from_redis_value(v: &redis::Value) -> RedisResult<Self> {
        match *v {
            Value::Bulk(ref items) => {
                let mut torrent = Torrent::default();
                let chunks = items.chunks(2);

                process_chunks_for_struct!(
                    chunks,
                    torrent,
                    TORRENT_COMPLETED_KEY => completed
                );

                Ok(torrent)
            }
            _ => Err((redis::ErrorKind::TypeError, "Unexpected type").into()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RedisConnectionManager {
    client: redis::Client,
}

impl RedisConnectionManager {
    pub fn new<T: IntoConnectionInfo>(params: T) -> Self {
        let client = redis::Client::open(params).expect("Invalid connection URL");
        Self { client }
    }
}

#[async_trait]
impl ManageConnection for RedisConnectionManager {
    type Connection = redis::aio::Connection;
    type Error = redis::RedisError;

    async fn connect(&self) -> std::result::Result<Self::Connection, Self::Error> {
        self.client.get_tokio_connection().await
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> std::result::Result<(), Self::Error> {
        let pong: String = redis::cmd("PING").query_async(conn).await?;
        match pong.as_str() {
            "PONG" => Ok(()),
            _ => Err((redis::ErrorKind::ResponseError, "ping request").into()),
        }
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        false
    }
}
