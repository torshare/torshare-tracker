pub mod full_scrape;

use self::full_scrape::FullScrapeCache;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;
use ts_utils::time::Instant;

/// A cache for storing various types of data, such as full scrape responses.
pub struct Cache {
    /// Cached data for full scrape responses, protected by a read-write lock.
    pub full_scrape: RwLock<CacheEntry<FullScrapeCache>>,
}

impl Cache {
    pub fn new() -> Cache {
        Cache {
            full_scrape: RwLock::new(CacheEntry::default()),
        }
    }
}

/// Represents an entry in the cache for storing data with expiration and refresh control.
///
/// The `CacheEntry` struct is a generic type that allows storing various types of data
/// in the cache along with metadata such as expiration time and refresh status.
#[derive(Default)]
pub struct CacheEntry<T> {
    /// The cached data stored within the entry.
    data: T,
    /// The instant at which the cached data expires (if applicable).
    expires: Option<Instant>,
    /// A flag indicating whether the cached data is being refreshed.
    refreshing: AtomicBool,
}

impl<T> CacheEntry<T>
where
    T: Send + Sync,
{
    pub fn new(data: T, expires: Option<Instant>) -> CacheEntry<T> {
        CacheEntry {
            data,
            expires,
            refreshing: AtomicBool::new(false),
        }
    }

    pub fn is_expired(&self) -> bool {
        match self.expires {
            Some(expires) => expires < Instant::now(),
            None => false,
        }
    }

    pub fn is_refreshing(&self) -> bool {
        self.refreshing.load(Ordering::SeqCst)
    }

    pub fn set_refreshing(&mut self) {
        self.refreshing.store(true, Ordering::SeqCst);
    }

    pub fn set(&mut self, data: T, expires: Option<Instant>) {
        debug_assert!(self.is_refreshing());

        self.data = data;
        self.expires = expires;
        self.refreshing.store(false, Ordering::SeqCst);
    }
}

impl<T> From<(T, Instant)> for CacheEntry<T>
where
    T: Send + Sync,
{
    fn from(value: (T, Instant)) -> CacheEntry<T> {
        CacheEntry::new(value.0, Some(value.1))
    }
}

impl<T> std::ops::Deref for CacheEntry<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use ts_utils::time::Duration;

    use super::*;

    #[test]
    fn test_cache_entry_is_expired() {
        let mut entry = CacheEntry::new((), None);
        assert!(!entry.is_expired());

        let expires = Instant::now() + Duration::from_secs(1);
        entry.expires = Some(expires);
        assert!(!entry.is_expired());

        let expires = Instant::now() - Duration::from_secs(1);
        entry.expires = Some(expires);
        assert!(entry.is_expired());
    }

    #[test]
    fn test_cache_entry_is_refreshing() {
        let mut entry = CacheEntry::new((), None);
        assert!(!entry.is_refreshing());

        entry.set_refreshing();
        assert!(entry.is_refreshing());
    }

    #[test]
    fn test_cache_entry_set() {
        let mut entry = CacheEntry::new((), None);
        assert!(!entry.is_refreshing());

        entry.set_refreshing();
        assert!(entry.is_refreshing());

        entry.set((), None);
        assert!(!entry.is_refreshing());
    }
}
