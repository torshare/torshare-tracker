use std::{ops::DerefMut, time::Duration};

use async_trait::async_trait;
use bytes::BytesMut;
use redis::{
    cmd, AsyncCommands, FromRedisValue, IntoConnectionInfo, RedisResult, ToRedisArgs, Value,
};
use ts_pool::{ManageConnection, Pool, PooledConnection};

use super::{Processor, Result, Storage};
use crate::{
    config::RedisStorageConfig,
    models::{
        common::{InfoHash, INFOHASH_LENGTH},
        peer::{Peer, PeerType},
        torrent::{PeerDict, PeerIdKey, TorrentStats, TorrentStatsDict},
    },
};

#[derive(Debug)]
pub struct RedisStorage {
    pool: Pool<RedisConnectionManager>,
}

impl RedisStorage {
    pub fn new(config: RedisStorageConfig) -> Self {
        let manager = RedisConnectionManager::new(config.url);
        let pool = Pool::builder()
            .max_size(1024)
            .idle_timeout(Duration::from_secs(10))
            .reaper_rate(Duration::from_secs(10))
            .test_on_check_out(false)
            .build(manager)
            .expect("Failed to create redis pool");

        Self { pool }
    }

    pub async fn get_connection(
        &self,
    ) -> RedisResult<PooledConnection<'_, RedisConnectionManager>> {
        match self.pool.get().await {
            Ok(Some(conn)) => Ok(conn),
            Ok(None) => Err((redis::ErrorKind::IoError, "failed to get connection").into()),
            Err(err) => Err(err.into()),
        }
    }
}

#[async_trait]
impl Storage for RedisStorage {
    async fn insert_torrent(
        &self,
        info_hash: &InfoHash,
        torrent: Option<TorrentStats>,
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;

        let _ = cmd("HMSET")
            .arg(info_hash)
            .arg(torrent.unwrap_or_default())
            .query_async(conn.deref_mut())
            .await?;

        Ok(())
    }

    async fn has_torrent(&self, info_hash: &InfoHash) -> Result<bool> {
        Ok(self.get_connection().await?.exists(info_hash).await?)
    }

    async fn remove_torrent(&mut self, info_hash: &InfoHash) -> Result<()> {
        Ok(self.get_connection().await?.del(info_hash).await?)
    }

    async fn get_torrent_stats(&self, info_hash: &InfoHash) -> Result<Option<TorrentStats>> {
        Ok(self.get_connection().await?.hgetall(info_hash).await?)
    }

    async fn get_multi_torrent_stats(
        &self,
        info_hashes: Vec<InfoHash>,
    ) -> Result<Vec<(InfoHash, TorrentStats)>> {
        let mut pipe = redis::pipe();
        let mut conn = self.get_connection().await?;

        for info_hash in &info_hashes {
            pipe.hgetall(info_hash);
        }

        let results: Vec<TorrentStats> = pipe.query_async(conn.deref_mut()).await?;

        Ok(info_hashes
            .into_iter()
            .zip(results.into_iter())
            .collect::<Vec<_>>())
    }

    async fn get_all_torrent_stats(
        &self,
        _processor: &mut dyn Processor<TorrentStatsDict>,
    ) -> Result<()> {
        Ok(())
    }

    async fn put_peer_in_swarm(
        &self,
        _info_hash: &InfoHash,
        _peer_id: &PeerIdKey,
        _peer: Peer,
        _peer_type: PeerType,
    ) -> Result<()> {
        Ok(())
    }

    async fn promote_peer_in_swarm(
        &self,
        _info_hash: &InfoHash,
        _peer_id: &PeerIdKey,
        _peer: Peer,
    ) -> Result<()> {
        unimplemented!()
    }

    async fn update_or_put_peer_in_swarm(
        &self,
        _info_hash: &InfoHash,
        _peer_id: &PeerIdKey,
        _peer: Peer,
        _peer_type: PeerType,
    ) -> Result<()> {
        unimplemented!()
    }

    async fn extract_peers_from_swarm(
        &self,
        _info_hash: &InfoHash,
        _peer_type: PeerType,
        _processor: &mut dyn Processor<PeerDict>,
    ) -> Result<()> {
        Ok(())
    }

    async fn remove_peer_from_swarm(
        &self,
        _info_hash: &InfoHash,
        _peer_id: &PeerIdKey,
        _peer_type: PeerType,
    ) -> Result<Option<Peer>> {
        unimplemented!()
    }
}

impl From<redis::RedisError> for super::Error {
    fn from(err: redis::RedisError) -> Self {
        Self::from(err.to_string())
    }
}

const TORRENT_KEY_PREFIX: &[u8] = b"ts_";
const TORRENT_KEY_LEN: usize = TORRENT_KEY_PREFIX.len() + INFOHASH_LENGTH * 2;

impl ToRedisArgs for InfoHash {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        let mut bytes = BytesMut::with_capacity(TORRENT_KEY_LEN);
        bytes.extend_from_slice(TORRENT_KEY_PREFIX);
        bytes.extend_from_slice(self.to_string().as_bytes());
        out.write_arg(&bytes);
    }
}

// Define the macro for serializing fields
macro_rules! serialize_field {
    ($out:ident, $field_name:expr, $field:expr) => {
        $field_name.write_redis_args($out);
        $field.write_redis_args($out);
    };
}

macro_rules! process_chunks_for_struct {
    ($chunks:expr, $struct:expr, $($pattern:expr => $field:ident),*) => {
        for item in $chunks {
            let field = item[0].clone();
            let value = item[1].clone();

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

impl ToRedisArgs for TorrentStats {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        serialize_field!(out, "s", self.seeders);
        serialize_field!(out, "c", self.completed);
        serialize_field!(out, "l", self.leechers);
    }

    fn is_single_arg(&self) -> bool {
        false
    }
}

impl FromRedisValue for TorrentStats {
    fn from_redis_value(v: &redis::Value) -> RedisResult<Self> {
        match *v {
            Value::Bulk(ref items) => {
                let mut stats = TorrentStats::default();
                let chunks = items.chunks(2);

                process_chunks_for_struct!(
                    chunks,
                    stats,
                    b"s" => seeders,
                    b"l" => leechers,
                    b"c" => completed
                );

                Ok(stats)
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
