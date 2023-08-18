mod cache;
mod http;
mod udp;

use crate::config::TSConfig;
use crate::worker::Worker;
use std::sync::Arc;

use self::cache::Cache;
pub use self::http::HttpServer;
pub use self::udp::UdpServer;

#[derive(Clone)]
pub struct State {
    pub worker: Arc<Worker>,
    pub config: Arc<TSConfig>,
    pub cache: Arc<Cache>,
}

impl State {
    pub fn new(worker: Arc<Worker>, config: Arc<TSConfig>) -> State {
        State {
            worker,
            config,
            cache: Arc::new(Cache::new()),
        }
    }
}
