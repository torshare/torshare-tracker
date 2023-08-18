use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use ts_utils::{
    serde::{deserialize_header_name, deserialize_option_string, deserialize_secs_to_duration},
    Set,
};

use crate::models::common::InfoHash;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    /// Storage implemented using in-memory data structures.
    Memory,
    /// Storage backed by the Redis key-value store.
    Redis,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MemoryStorageConfig {}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RedisStorageConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StorageConfig {
    #[serde(rename = "type")]
    pub name: StorageType,
    pub redis: Option<RedisStorageConfig>,
    pub memory: Option<MemoryStorageConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub http: HttpServerConfig,
    pub udp: UdpServerConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
/// Configuration options for an HTTP server.
pub struct HttpServerConfig {
    /// The port number on which the HTTP server will listen for incoming requests.
    pub port: u16,

    /// The host address of the HTTP server.
    pub host: String,

    /// Determines whether to log incoming requests.
    pub log_request: bool,

    /// Determines whether to enable HTTP keep-alive connections.
    pub enable_keep_alive: bool,

    #[serde(deserialize_with = "deserialize_secs_to_duration")]
    /// The duration of time for an idle keep-alive connection before it's closed.
    pub keep_alive_idle_time: Duration,

    /// The maximum buffer size for reading incoming request data.
    pub max_read_buffer_size: usize,

    #[serde(deserialize_with = "deserialize_secs_to_duration")]
    /// The maximum allowed duration for processing an incoming request.
    pub request_timeout: Duration,

    #[serde(deserialize_with = "deserialize_header_name")]
    /// The header name used to forward IP address information (optional).
    pub ip_forward_header_name: Option<String>,

    #[serde(deserialize_with = "deserialize_option_string")]
    /// The API key used for performing tracker API calls (optional).
    pub api_key: Option<String>,

    /// Determines whether to enable GZIP compression for scrape responses.
    pub gzip_scrape: bool,

    /// The size of the connection backlog for incoming requests.
    pub connection_backlog_size: usize,

    /// The maximum number of concurrent requests the server can handle.
    pub max_concurrent_requests: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
/// Configuration options for a UDP server.
pub struct UdpServerConfig {
    /// The port number on which the UDP server will listen for incoming messages.
    pub port: u16,

    /// The host address of the UDP server.
    pub host: String,

    #[serde(deserialize_with = "deserialize_option_string")]
    /// An optional secret key used for authentication and security.
    pub secret_key: Option<String>,
}

/// Configuration options for a BitTorrent tracker.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TrackerConfig {
    /// Determines whether torrents are automatically registered upon announce.
    pub auto_register_torrent: bool,

    /// The interval at which clients should announce their status to the tracker.
    pub announce_interval: u32,

    /// The minimum interval allowed between client announces.
    pub min_announce_interval: u32,

    /// The interval at which clients should scrape the tracker for information.
    pub scrape_interval: u32,

    /// Determines whether a full scrape is allowed.
    pub allow_full_scrape: bool,

    /// The duration of time for which a full scrape is cached.
    #[serde(deserialize_with = "deserialize_secs_to_duration")]
    pub full_scrape_cache_ttl: Duration,

    /// The maximum number of torrents to scrape in a single request.
    pub max_multi_scrape_count: u32,

    /// The maximum number of peers to include in a response to an announce request.
    pub max_numwant: u32,

    /// The default number of peers to include in a response to an announce request.
    pub default_numwant: u32,

    /// Determines whether UDP announce requests are allowed.
    pub allow_udp_announce: bool,

    /// Determines whether HTTP announce requests are allowed.
    pub allow_http_announce: bool,

    /// Determines whether UDP scrape requests are allowed.
    pub allow_udp_scrape: bool,

    /// Determines whether HTTP scrape requests are allowed.
    pub allow_http_scrape: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
/// Configuration options for a TS tracker server.
pub struct TSConfig {
    /// Configuration options for the server component.
    pub server: ServerConfig,
    /// Configuration options for the tracker component.
    pub tracker: TrackerConfig,
    /// Configuration options for the storage component.
    pub storage: StorageConfig,
    /// The log level to control the verbosity of log messages.
    pub log_level: String,

    #[serde(deserialize_with = "deserialize_option_string")]
    infohash_blocklist_file: Option<String>,

    #[serde(skip)]
    pub infohash_blocklist: InfoHashBlockList,
}

impl TSConfig {
    /// Attempt to load the configuration from the environment.
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = Config::builder()
            // Add in a default configuration file
            .add_source(File::with_name("conf/default").required(true))

            // Add in the current environment file
            // Default to 'development' env
            .add_source(
                File::with_name(&format!("conf/{}", run_mode))
                    .required(false),
            )

            // Add in a local configuration file
            // This file shouldn't be checked in to git
            .add_source(File::with_name("conf/local").required(false))

            // Add in settings from the environment (with a prefix of TS)
            // Eg.. `TS_LOG_LEVEL=debug ./target/app` would set the `log_level` key
            .add_source(Environment::with_prefix("ts"))
            .build()?;

        let mut config: TSConfig = s.try_deserialize()?;

        // Load the infohash blocklist if a file path is specified
        if let Some(ref file_path) = config.infohash_blocklist_file {
            config
                .infohash_blocklist
                .load(&file_path)
                .expect("failed to load infohash blocklist");
        }

        Ok(config)
    }

    pub fn http_request_timeout(&self) -> Duration {
        self.server.http.request_timeout
    }

    pub fn max_read_buffer_size(&self) -> usize {
        self.server.http.max_read_buffer_size
    }

    pub fn http_log_request(&self) -> bool {
        self.server.http.log_request
    }

    pub fn auto_register_torrent(&self) -> bool {
        self.tracker.auto_register_torrent
    }

    pub fn http_port(&self) -> u16 {
        self.server.http.port
    }

    pub fn http_host(&self) -> &str {
        self.server.http.host.as_ref()
    }

    pub fn udp_port(&self) -> u16 {
        self.server.udp.port
    }

    pub fn udp_host(&self) -> &str {
        self.server.udp.host.as_ref()
    }

    pub fn ip_forward_header_name(&self) -> Option<&String> {
        self.server.http.ip_forward_header_name.as_ref()
    }

    pub fn api_key(&self) -> Option<&String> {
        self.server.http.api_key.as_ref()
    }

    pub fn is_keep_alive_enabled(&self) -> bool {
        self.server.http.enable_keep_alive
    }

    pub fn keep_alive_idle_time(&self) -> Duration {
        self.server.http.keep_alive_idle_time
    }

    pub fn max_open_connections(&self) -> usize {
        self.server.http.max_concurrent_requests
    }

    pub fn connection_backlog_size(&self) -> i32 {
        self.server.http.connection_backlog_size as i32
    }

    pub fn max_numwant(&self) -> u32 {
        self.tracker.max_numwant
    }

    pub fn default_numwant(&self) -> u32 {
        self.tracker.default_numwant
    }

    pub fn announce_interval(&self) -> u32 {
        self.tracker.announce_interval
    }

    pub fn min_announce_interval(&self) -> u32 {
        self.tracker.min_announce_interval
    }

    pub fn scrape_interval(&self) -> u32 {
        self.tracker.scrape_interval
    }

    pub fn allow_full_scrape(&self) -> bool {
        self.tracker.allow_full_scrape
    }

    pub fn allow_udp_announce(&self) -> bool {
        self.tracker.allow_udp_announce
    }

    pub fn allow_http_announce(&self) -> bool {
        self.tracker.allow_http_announce
    }

    pub fn allow_udp_scrape(&self) -> bool {
        self.tracker.allow_udp_scrape
    }

    pub fn allow_http_scrape(&self) -> bool {
        self.tracker.allow_http_scrape
    }

    pub fn log_level(&self) -> &str {
        self.log_level.as_ref()
    }

    pub fn max_multi_scrape_count(&self) -> u32 {
        self.tracker.max_multi_scrape_count
    }

    pub fn full_scrape_cache_ttl(&self) -> Duration {
        self.tracker.full_scrape_cache_ttl
    }
}

#[derive(Debug, Default, Clone)]
pub struct InfoHashBlockList(Set<InfoHash>);

impl InfoHashBlockList {
    fn load(&mut self, file_path: &str) -> std::io::Result<()> {
        self.0.load_from_file(file_path)
    }
}

impl std::ops::Deref for InfoHashBlockList {
    type Target = Set<InfoHash>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
