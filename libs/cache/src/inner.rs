use std::hash::Hash;
use std::{fmt, sync::Arc};
use tokio::sync::{mpsc, oneshot};

use crate::api::CacheKey;
use crate::{
    internals::{CacheInternal, Message},
    Builder, CacheLoader,
};

pub(crate) struct CacheInner<K: Clone, V> {
    tx: Arc<mpsc::Sender<Message<K, V>>>,
}

impl<K, V> CacheInner<K, V>
where
    K: Eq + Hash + Send + Clone + Sync + 'static,
    V: Send + Sync + Clone + 'static,
{
    pub(super) fn new(statics: Builder, loader: Arc<dyn CacheLoader<Key = K, Value = V>>) -> Self {
        let (tx, rx) = mpsc::channel(128);
        let tx = Arc::new(tx);
        let weak_tx = Arc::downgrade(&tx);

        tokio::spawn(async move {
            let internal = CacheInternal::new(statics, loader);
            internal.run(rx, weak_tx).await
        });

        Self { tx }
    }

    pub(crate) async fn get(&self, key: CacheKey<'_, K>) -> Option<V> {
        let (tx, rx) = oneshot::channel();

        if self
            .tx
            .send(Message::Get((key.into_owned(), tx)))
            .await
            .is_err()
        {
            return None;
        }

        match rx.await {
            Ok(Some(value)) => Some(value),
            _ => None,
        }
    }

    pub(crate) async fn get_all(&self, keys: Vec<K>) -> Vec<(K, V)> {
        let (tx, rx) = oneshot::channel();
        if self.tx.send(Message::GetAll((keys, tx))).await.is_err() {
            return Vec::new();
        }

        match rx.await {
            Ok(value) => value,
            _ => Vec::new(),
        }
    }

    pub(crate) async fn set(&self, key: K, value: V) {
        let _ = self.tx.send(Message::Set(key, value)).await;
    }

    pub(crate) async fn remove(&self, key: CacheKey<'_, K>) -> Option<V> {
        let (tx, rx) = oneshot::channel();
        if self
            .tx
            .send(Message::Remove((key.into_owned(), tx)))
            .await
            .is_err()
        {
            return None;
        }

        match rx.await {
            Ok(Some(value)) => Some(value),
            _ => None,
        }
    }

    pub(crate) async fn remove_all(&self, keys: Vec<K>) {
        let _ = self.tx.send(Message::RemoveAll(keys)).await;
    }

    pub(crate) async fn clear(&self) {
        let _ = self.tx.send(Message::Clear).await;
    }
}

impl<K: Clone, V> fmt::Debug for CacheInner<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CacheInner").finish()
    }
}
