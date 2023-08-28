use ahash::RandomState;
use std::hash::Hash;
use std::sync::{Arc, Weak};
use std::time::Duration;
use std::{collections::HashMap, time::Instant};
use tokio::sync::{mpsc, oneshot, watch};

use crate::api::CacheLoader;
use crate::{Builder, Policy};

type SendValueTx<V> = oneshot::Sender<Option<V>>;
type SendMultipleValueTx<K, V> = oneshot::Sender<Vec<(K, V)>>;
type CacheRx<K, V> = mpsc::Receiver<Message<K, V>>;
type WeakCacheTx<K, V> = Weak<mpsc::Sender<Message<K, V>>>;
type WatchTx<V> = watch::Sender<Option<V>>;
type WatchRx<V> = watch::Receiver<Option<V>>;
type Loader<K, V> = Arc<dyn CacheLoader<Key = K, Value = V>>;

pub(crate) enum Message<K, V> {
    Get((K, SendValueTx<V>)),
    GetAll((Vec<K>, SendMultipleValueTx<K, V>)),
    Set(K, V),
    Remove((K, SendValueTx<V>)),
    RemoveAll(Vec<K>),
    Clear,
    Load(K, Option<V>, WatchTx<V>),
}

pub(crate) struct CacheInternal<K, V> {
    map: HashMap<K, CacheEntry<V>, RandomState>,
    loader: Loader<K, V>,
    statics: Builder,
}

impl<K, V> CacheInternal<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Send + Sync + Clone + 'static,
{
    pub(crate) fn new(statics: Builder, loader: Arc<dyn CacheLoader<Key = K, Value = V>>) -> Self {
        Self {
            map: HashMap::with_hasher(RandomState::default()),
            loader,
            statics,
        }
    }

    pub(crate) fn insert(&mut self, key: K, value: Option<V>) {
        self.map
            .insert(key, CacheEntry::new_with_expiry(value, self.statics.expiry));
    }

    pub(crate) async fn run(
        mut self: CacheInternal<K, V>,
        mut rx: CacheRx<K, V>,
        weak_tx: WeakCacheTx<K, V>,
    ) {
        while let Some(msg) = rx.recv().await {
            match msg {
                Message::Get((key, tx)) => match self.map.get_mut(&key) {
                    Some(entry) => {
                        if entry.is_loading() {
                            entry.wait_for_value(tx);
                            continue;
                        }

                        Self::get_or_load_entry(
                            entry,
                            key,
                            self.loader.clone(),
                            tx,
                            weak_tx.clone(),
                            self.statics.policy,
                        );
                    }
                    None => {
                        let mut entry = CacheEntry::default();
                        let watch_tx = entry.init_watch_and_wait(tx);

                        Self::load_entry(
                            &mut entry,
                            key.clone(),
                            self.loader.clone(),
                            weak_tx.clone(),
                            watch_tx,
                        );

                        self.map.insert(key, entry);
                    }
                },

                Message::Set(key, value) => {
                    let _ = self.insert(key, Some(value));
                }

                Message::Remove((key, tx)) => {
                    let value = self.map.remove(&key);
                    let _ = tx.send(value.and_then(|v| v.value));
                }

                Message::RemoveAll(keys) => {
                    for key in keys {
                        self.map.remove(&key);
                    }
                }

                Message::Clear => {
                    self.map.clear();
                }

                Message::Load(key, value, tx) => match self.map.contains_key(&key) {
                    true => {
                        let _ = tx.send(value.clone());
                        let _ = self.insert(key, value);
                    }
                    false => {
                        let _ = tx.send(None);
                    }
                },

                Message::GetAll((_keys, tx)) => {
                    let _ = tx.send(Vec::new());
                }
            }
        }
    }

    fn load_entry(
        entry: &mut CacheEntry<V>,
        key: K,
        loader: Loader<K, V>,
        weak_cache_tx: WeakCacheTx<K, V>,
        watch_tx: WatchTx<V>,
    ) {
        entry.state = match entry.state {
            CacheEntryState::Init => CacheEntryState::Loading,
            _ => CacheEntryState::Refreshing,
        };

        entry.load(key, loader, weak_cache_tx, watch_tx);
    }

    fn get_or_load_entry(
        entry: &mut CacheEntry<V>,
        key: K,
        loader: Loader<K, V>,
        tx: SendValueTx<V>,
        weak_cache_tx: WeakCacheTx<K, V>,
        policy: Policy,
    ) {
        match policy {
            Policy::RefreshAfterAccess if entry.is_expired() => {
                entry.state = CacheEntryState::Expired;
                let watch_tx = entry.init_watch();
                Self::load_entry(entry, key, loader, weak_cache_tx, watch_tx);
                let _ = tx.send(entry.value.clone());
            }

            Policy::RefreshAfterAccess => {
                let _ = tx.send(entry.value.clone());
            }

            Policy::RefreshBeforeAccess if entry.is_refreshing() => {
                entry.wait_for_value(tx);
            }

            Policy::RefreshBeforeAccess if entry.is_expired() => {
                entry.state = CacheEntryState::Expired;
                let watch_tx = entry.init_watch_and_wait(tx);
                Self::load_entry(entry, key, loader, weak_cache_tx, watch_tx);
            }

            Policy::RefreshBeforeAccess => {
                let _ = tx.send(entry.value.clone());
            }
        }
    }
}

struct CacheEntry<V> {
    value: Option<V>,
    expire_at: Option<Instant>,
    watch_rx: Option<WatchRx<V>>,
    state: CacheEntryState,
}

impl<V: Clone + Send + Sync + 'static> CacheEntry<V> {
    fn new(value: Option<V>, expire_at: Instant) -> Self {
        Self {
            value,
            expire_at: Some(expire_at),
            watch_rx: None,
            state: CacheEntryState::Valid,
        }
    }

    fn new_with_expiry(value: Option<V>, expiry: Duration) -> Self {
        Self::new(value, Instant::now() + expiry)
    }

    fn is_expired(&mut self) -> bool {
        if self.state == CacheEntryState::Expired {
            return true;
        }

        if let Some(expire_at) = self.expire_at {
            return Instant::now() > expire_at;
        }

        false
    }

    fn is_refreshing(&self) -> bool {
        debug_assert!(self.watch_rx.is_some());
        self.state == CacheEntryState::Refreshing
    }

    fn is_loading(&self) -> bool {
        debug_assert!(self.watch_rx.is_some());
        self.state == CacheEntryState::Loading
    }

    fn load<K: Send + Sync + Clone + 'static>(
        &mut self,
        key: K,
        loader: Arc<dyn CacheLoader<Key = K, Value = V>>,
        weak_tx: WeakCacheTx<K, V>,
        watch_tx: WatchTx<V>,
    ) {
        tokio::spawn(async move {
            let value = loader.load(&key).await;
            match weak_tx.upgrade() {
                Some(tx) => match tx.send(Message::Load(key, value, watch_tx)).await {
                    Ok(_) => {}
                    Err(err) => match err.0 {
                        Message::Load(_, _, watch_tx) => {
                            let _ = watch_tx.send(None);
                        }
                        _ => unreachable!(),
                    },
                },
                None => {
                    let _ = watch_tx.send(None);
                }
            };
        });
    }

    fn wait_for_value(&self, tx: SendValueTx<V>) {
        let mut watch_rx = match self.watch_rx {
            Some(ref rx) => rx.clone(),
            None => {
                let _ = tx.send(None);
                return;
            }
        };

        tokio::spawn(async move {
            match watch_rx.changed().await {
                Ok(_) => {
                    let _ = match watch_rx.borrow().as_ref() {
                        Some(value) => tx.send(Some(value.clone())),
                        None => tx.send(None),
                    };
                }
                Err(_) => {
                    let _ = tx.send(None);
                }
            }
        });
    }

    fn init_watch(&mut self) -> WatchTx<V> {
        let (tx, rx) = watch::channel(None);
        self.watch_rx = Some(rx);
        tx
    }

    fn init_watch_and_wait(&mut self, tx: SendValueTx<V>) -> WatchTx<V> {
        let watch_tx = self.init_watch();
        self.wait_for_value(tx);
        watch_tx
    }
}

impl<V> Default for CacheEntry<V> {
    fn default() -> Self {
        Self {
            value: None,
            expire_at: None,
            watch_rx: None,
            state: CacheEntryState::Init,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CacheEntryState {
    Init,
    Valid,
    Loading,
    Refreshing,
    Expired,
}
