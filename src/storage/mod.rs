use crate::config::StorageType;
use crate::models::common::InfoHash;
use crate::models::peer::{Peer, PeerType};
use crate::models::torrent::{PeerDict, PeerIdKey, TorrentStats, TorrentStatsDict};
use async_trait::async_trait;
use std::error::Error as StdError;
use std::fmt;

mod memory;
pub use self::memory::MemoryStorage;

#[cfg(feature = "redis-store")]
mod redis;

#[cfg(feature = "redis-store")]
pub use self::redis::RedisStorage;

#[async_trait]
pub trait Storage: Sync + Send {
    async fn insert_torrent(&self, info_hash: &InfoHash, stats: Option<TorrentStats>)
        -> Result<()>;

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

pub fn create_new_storage(config: &crate::config::TSConfig) -> Result<Box<dyn Storage>> {
    let storage_type = config.storage.name.to_owned();
    log::info!("Storage type: {:?}", storage_type);

    match storage_type {
        StorageType::Memory => Ok(Box::new(MemoryStorage::new())),
        #[cfg(feature = "redis-store")]
        StorageType::Redis => {
            let redis_config = config.storage.redis.clone().unwrap();
            Ok(Box::new(RedisStorage::new(redis_config)))
        }
        _ => Err("Unsupported storage type".into()),
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

type Cause = Box<dyn StdError + Send + Sync>;
pub struct Error {
    inner: Box<ErrorImpl>,
}

struct ErrorImpl {
    kind: Kind,
    cause: Option<Cause>,
}

#[derive(Debug)]
enum Kind {
    Custom(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<&str> for Error {
    fn from(err: &str) -> Self {
        err.to_string().into()
    }
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Self {
            inner: Box::new(ErrorImpl {
                kind: Kind::Custom(err),
                cause: None,
            }),
        }
    }
}

impl Error {
    /// The error's standalone message, without the message from the source.
    pub fn message(&self) -> impl fmt::Display + '_ {
        self.description()
    }

    fn description(&self) -> &str {
        match self.inner.kind {
            Kind::Custom(ref msg) => msg,
        }
    }
}

impl StdError for Error {}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_tuple("storage::Error");
        f.field(&self.inner.kind);
        if let Some(ref cause) = self.inner.cause {
            f.field(cause);
        }
        f.finish()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.description())
    }
}
