use crate::config::{StorageType, TSConfig};
use crate::models::common::{InfoHash, IpType};
use crate::models::peer::{Peer, PeerType};
use crate::models::torrent::{
    PeerDict, PeerIdKey, PeerList, SwarmStats, Torrent, TorrentStats, TorrentStatsList,
};
use async_trait::async_trait;
use std::error::Error as StdError;
use std::fmt;
use std::sync::Arc;

mod memory;
pub use self::memory::MemoryStorage;

#[cfg(feature = "redis-store")]
mod redis;

#[cfg(feature = "redis-store")]
pub use self::redis::RedisStorage;

#[async_trait]
pub trait Storage: Sync + Send {
    async fn insert_torrent(&self, info_hash: &InfoHash, stats: Option<Torrent>) -> Result<()>;
    async fn remove_torrent(&mut self, info_hash: &InfoHash) -> Result<()>;
    async fn has_torrent(&self, info_hash: &InfoHash) -> Result<bool>;
    async fn get_torrent(&self, info_hash: &InfoHash) -> Result<Option<Torrent>>;

    async fn get_torrent_stats(
        &self,
        info_hash: &InfoHash,
        ip_type: IpType,
    ) -> Result<TorrentStats>;

    async fn get_multi_torrent_stats(
        &self,
        info_hashes: Vec<InfoHash>,
        ip_type: IpType,
    ) -> Result<TorrentStatsList>;

    async fn get_all_torrent_stats(
        &self,
        processor: &mut dyn Processor<TorrentStatsList>,
    ) -> Result<()>;

    async fn put_peer_in_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id_key: &PeerIdKey,
        peer: Peer,
        peer_type: PeerType,
    ) -> Result<()>;

    async fn update_or_put_peer_in_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id_key: &PeerIdKey,
        peer: Peer,
        peer_type: PeerType,
    ) -> Result<()>;

    async fn promote_peer_in_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id_key: &PeerIdKey,
        peer: Peer,
    ) -> Result<()>;

    async fn extract_peers_from_swarm(
        &self,
        info_hash: &InfoHash,
        peer_type: PeerType,
        peer_ip_type: IpType,
        extractor: &mut dyn PeerExtractor,
    ) -> Result<SwarmStats>;

    async fn remove_peer_from_swarm(
        &self,
        info_hash: &InfoHash,
        peer_id_key: &PeerIdKey,
        peer_type: PeerType,
        peer_ip_type: IpType,
    ) -> Result<()>;
}

pub fn create_new_storage(config: Arc<TSConfig>) -> Result<Box<dyn Storage>> {
    let storage_type = config.storage.name.to_owned();
    log::info!("Storage type: {:?}", storage_type);

    match storage_type {
        StorageType::Memory => {
            let shard_count = config.storage.memory.as_ref().unwrap();
            Ok(Box::new(MemoryStorage::with_shards(
                shard_count.shard_count as usize,
            )))
        }
        #[cfg(feature = "redis-store")]
        StorageType::Redis => Ok(Box::new(RedisStorage::new(config))),
        #[cfg(not(feature = "redis-store"))]
        _ => Err("Unsupported storage type".into()),
    }
}

type Cause = Box<dyn StdError + Send + Sync>;

pub struct Error {
    inner: Box<ErrorImpl>,
}

impl Error {
    #[allow(unused)]
    fn runtime(cause: Cause) -> Self {
        Self {
            inner: Box::new(ErrorImpl {
                kind: Kind::Runtime(None),
                cause: Some(cause),
            }),
        }
    }
}

struct ErrorImpl {
    kind: Kind,
    cause: Option<Cause>,
}

#[derive(Debug)]
#[allow(unused)]
enum Kind {
    Runtime(Option<String>),
    Custom(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<&'static str> for Error {
    fn from(err: &'static str) -> Self {
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
            Kind::Runtime(ref msg) => msg.as_deref().unwrap_or_default(),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.inner.cause.as_ref().map(|cause| &**cause as _)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref cause) = self.inner.cause {
            write!(f, "{}: {}", self.description(), cause)
        } else {
            f.write_str(self.description())
        }
    }
}

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

pub trait PeerExtractor: Send {
    fn from_dict(&mut self, dict: &PeerDict) -> bool;
    fn from_list(&mut self, list: &PeerList) -> bool;
}
