use async_trait::async_trait;
use redis::RedisResult;

use super::{Processor, Result, Storage};
use crate::{
    config::RedisStorageConfig,
    models::{
        common::InfoHash,
        peer::{Peer, PeerType},
        torrent::{PeerDict, PeerIdKey, TorrentStats, TorrentStatsDict},
    },
};

#[derive(Debug)]
pub struct RedisStorage {
    client: redis::Client,
}

impl RedisStorage {
    pub fn new(config: RedisStorageConfig) -> Self {
        let client = redis::Client::open(config.url).expect("Failed to connect to Redis");
        Self { client }
    }

    pub async fn get_connection(&self) -> RedisResult<redis::aio::Connection> {
        self.client.get_async_connection().await
    }
}

#[async_trait]
impl Storage for RedisStorage {
    async fn insert_torrent(
        &self,
        _info_hash: &InfoHash,
        _torrent_stats: Option<TorrentStats>,
    ) -> Result<()> {
        unimplemented!()
    }

    async fn has_torrent(&self, _info_hash: &InfoHash) -> Result<bool> {
        unimplemented!()
    }

    async fn remove_torrent(&mut self, _info_hash: &InfoHash) -> Result<()> {
        unimplemented!()
    }

    async fn get_torrent_stats(&self, _info_hash: &InfoHash) -> Result<Option<TorrentStats>> {
        unimplemented!()
    }

    async fn get_multi_torrent_stats(
        &self,
        _info_hashes: Vec<InfoHash>,
    ) -> Result<Vec<(InfoHash, TorrentStats)>> {
        unimplemented!()
    }

    async fn get_all_torrent_stats(
        &self,
        _processor: &mut dyn Processor<TorrentStatsDict>,
    ) -> Result<()> {
        unimplemented!()
    }

    async fn put_peer_in_swarm(
        &self,
        _info_hash: &InfoHash,
        _peer_id: &PeerIdKey,
        _peer: Peer,
        _peer_type: PeerType,
    ) -> Result<()> {
        unimplemented!()
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
        unimplemented!()
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
