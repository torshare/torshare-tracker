use async_trait::async_trait;
use std::{borrow::Cow, hash::Hash, sync::Arc, time::Duration};

use crate::inner::CacheInner;

#[derive(Debug)]
pub struct Cache<K: Clone, V> {
    pub(crate) inner: Arc<CacheInner<K, V>>,
}

impl<K, V> Cache<K, V>
where
    K: Eq + Hash + Send + Clone + Sync + 'static,
    V: Send + Sync + Clone + 'static,
{
    /// Creates a new `Cache` instance with the provided builder and loader.
    pub(crate) fn new(statics: Builder, loader: Arc<dyn CacheLoader<Key = K, Value = V>>) -> Self {
        let inner = Arc::new(CacheInner::new(statics, loader));
        Self { inner }
    }

    /// Returns a builder for creating a new `Cache` instance.
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// Gets a value from the cache based on the given key.
    pub async fn get(&self, key: CacheKey<'_, K>) -> Option<V> {
        self.inner.get(key).await
    }

    pub async fn get_all<T>(&self, keys: T) -> Vec<(K, V)>
    where
        T: Iterator<Item = K>,
    {
        self.inner.get_all(keys.collect()).await
    }

    /// Sets a value in the cache with the specified key and value.
    pub async fn set(&self, key: K, value: V) {
        self.inner.set(key, value).await
    }

    /// Discards any cached value for key.
    pub async fn invalidate(&self, key: CacheKey<'_, K>) -> Option<V> {
        self.inner.remove(key).await
    }

    /// Discards any cached values for the given keys.
    pub async fn invalidate_all_keys<T>(&self, keys: T)
    where
        T: Iterator<Item = K>,
    {
        self.inner.remove_all(keys.collect()).await
    }

    /// Discards all entries in the cache.
    pub async fn invalidate_all(&self) {
        self.inner.clear().await
    }
}

/// An async trait for loading values into the cache.
///
/// This trait defines methods for loading values into the cache
/// based on provided keys.
#[async_trait]
pub trait CacheLoader: Send + Sync {
    type Key;
    type Value;

    async fn load(&self, key: &Self::Key) -> Option<Self::Value>;
    async fn load_all(&self, keys: &[Self::Key]) -> Vec<(Self::Key, Self::Value)>;
}

#[derive(Debug, Clone)]
pub struct CacheKey<'a, K: Clone>(Cow<'a, K>);

impl<'a, K: Clone> CacheKey<'a, K> {
    pub fn into_owned(self) -> K {
        self.0.into_owned()
    }
}

impl<'a, K: Clone> From<K> for CacheKey<'a, K> {
    fn from(key: K) -> Self {
        Self(Cow::Owned(key))
    }
}

impl<'a, K: Clone> From<&'a K> for CacheKey<'a, K> {
    fn from(key: &'a K) -> Self {
        Self(Cow::Borrowed(key))
    }
}

impl<'a, K: Clone> From<Cow<'a, K>> for CacheKey<'a, K> {
    fn from(key: Cow<'a, K>) -> Self {
        Self(key)
    }
}

impl<'a, K: Clone> std::ops::Deref for CacheKey<'a, K> {
    type Target = K;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A builder for creating `Cache` instances.
pub struct Builder {
    pub(crate) max_capacity: usize,
    pub(crate) expiry: Duration,
    pub(crate) policy: Policy,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            max_capacity: 1_024,
            expiry: Duration::from_secs(10),
            policy: Policy::RefreshAfterAccess,
        }
    }
}

impl Builder {
    /// Sets the max capacity of the cache being built.
    pub fn max_capacity(mut self, capacity: usize) -> Self {
        self.max_capacity = capacity;
        self
    }

    /// Sets the expiration duration for cache entries.
    pub fn expiry(mut self, expiry: Duration) -> Self {
        self.expiry = expiry;
        self
    }

    /// Sets the policy for refreshing cache entries.
    pub fn policy(mut self, policy: Policy) -> Self {
        self.policy = policy;
        self
    }

    /// Builds a new `Cache` instance using the provided loader and builder settings.
    pub fn build<K, V>(self, loader: Arc<dyn CacheLoader<Key = K, Value = V>>) -> Cache<K, V>
    where
        K: Eq + Hash + Send + Clone + Sync + 'static,
        V: Send + Sync + Clone + 'static,
    {
        Cache::new(self, loader)
    }
}

#[derive(Debug, Clone, Copy)]
/// Defines different cache policies for refreshing cache entries.
pub enum Policy {
    /// Indicates that expired cache entries should be refreshed after they are accessed.
    RefreshAfterAccess,
    /// Specifies that expired cache entries should be refreshed before they are accessed.
    RefreshBeforeAccess,
}
