use bytes::Bytes;
use std::sync::Arc;
use ts_utils::{
    time::{Duration, Instant},
    Shared,
};

use super::Cache;
use crate::{
    models::{torrent::TorrentStatsList, tracker::FullScrapeResponse},
    storage::Processor,
    worker::{FullScrapeProcessor, Task, TaskOutput, Worker},
};

/// A cache for storing data related to full scrape responses for a BitTorrent tracker.
#[derive(Debug, Default)]
pub struct FullScrapeCache {
    data: Option<Shared<bytes::Bytes>>,
}

impl FullScrapeCache {
    pub fn new(data: bytes::Bytes) -> FullScrapeCache {
        FullScrapeCache {
            data: Some(Shared::new(data)),
        }
    }
}

/// Asynchronously refreshes a cache using a worker, extending its validity period.
///
/// # Arguments
///
/// * `cache` - An `Arc` reference to the cache to be refreshed.
/// * `worker` - An `Arc` reference to the worker responsible for refreshing the cache.
/// * `expires_in` - The new validity duration to apply after the refresh operation.
pub async fn refresh(cache: Arc<Cache>, worker: Arc<Worker>, expires_in: Duration) {
    let should_refresh = {
        let mut cache = cache.full_scrape.write().await;
        match cache.as_ref() {
            Some(_) if cache.is_expired() && !cache.is_refreshing() => {
                cache.set_refreshing();
                true
            }
            None => {
                cache.set_refreshing();
                true
            }
            _ => false,
        }
    };

    if !should_refresh {
        return;
    }

    let task = Task::FullScrape(Box::new(FullScrapeResponse::new()));
    let data = match worker.work(task).await {
        Ok(TaskOutput::FullScrape(mut handler)) => handler.output().unwrap_or_default(),
        _ => Bytes::new(),
    };

    let mut cache = cache.full_scrape.write().await;

    cache.set(
        FullScrapeCache::new(data),
        Some(Instant::now() + expires_in),
    );
}

impl std::ops::Deref for FullScrapeCache {
    type Target = Option<Shared<bytes::Bytes>>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl std::ops::DerefMut for FullScrapeCache {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl FullScrapeProcessor for FullScrapeResponse {
    fn as_processor(&mut self) -> &mut dyn Processor<TorrentStatsList> {
        self
    }

    fn output(&mut self) -> Option<Bytes> {
        self.output()
    }
}

impl Processor<TorrentStatsList> for FullScrapeResponse {
    fn process(&mut self, input: &TorrentStatsList) -> bool {
        self.bencode(input.iter());
        return true;
    }
}
