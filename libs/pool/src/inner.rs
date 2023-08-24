use std::{
    fmt,
    sync::{Arc, Weak},
    time::Instant,
};
use tokio::{
    sync::{mpsc, oneshot},
    time::{interval_at, Interval},
};

use crate::{
    api::{Builder, PoolError},
    internals::{Conn, Message, SharedPool, State},
    ManageConnection,
};

pub(crate) struct PoolInner<M>
where
    M: ManageConnection + Send,
{
    channel: Arc<mpsc::Sender<Message<M>>>,
}

impl<M> PoolInner<M>
where
    M: ManageConnection + Send,
{
    /// Create a new `PoolInner`
    pub(crate) fn new(builder: Builder<M>, manager: M) -> Self {
        let idle_timeout = builder.idle_timeout;
        let reaper_rate = builder.reaper_rate;

        let channel = SharedPool::channel(manager, builder);

        if idle_timeout.is_some() {
            let ws = Arc::downgrade(&channel);
            let start = Instant::now() + reaper_rate;
            let interval = interval_at(start.into(), reaper_rate);

            schedule_reaping(interval, ws);
        }

        Self { channel }
    }

    /// Return connection back in to the pool
    pub(crate) fn put_back(&self, conn: Option<Conn<M::Connection>>) {
        let tx = self.channel.as_ref().clone();

        tokio::spawn(async move {
            match conn {
                Some(conn) => tx.send(Message::PutConn(conn)).await,
                None => Ok(()),
            }
        });
    }

    /// Get a connection from the pool
    pub(crate) async fn get(&self) -> Result<Option<Conn<M::Connection>>, PoolError<M::Error>> {
        let (tx, rx) = oneshot::channel();

        if self.channel.send(Message::GetConn(tx)).await.is_err() {
            return Ok(None);
        }

        match rx.await {
            Ok(result) => result,
            Err(_) => Ok(None),
        }
    }

    /// Get the current state of the pool
    pub(crate) async fn state(&self) -> Option<State> {
        let (tx, rx) = oneshot::channel();

        if self.channel.send(Message::State(tx)).await.is_err() {
            return None;
        }

        match rx.await {
            Ok(state) => Some(state),
            Err(_) => None,
        }
    }

    /// Reap connections from the pool
    pub(crate) fn reap(&self) {
        let tx = self.channel.as_ref().clone();
        tokio::spawn(async move {
            let _ = tx.send(Message::Reap).await;
        });
    }
}

impl<M> Clone for PoolInner<M>
where
    M: ManageConnection + Send,
{
    fn clone(&self) -> Self {
        Self {
            channel: self.channel.clone(),
        }
    }
}

impl<M> fmt::Debug for PoolInner<M>
where
    M: ManageConnection,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PoolInner").finish()
    }
}

fn schedule_reaping<M>(mut interval: Interval, weak_shared: Weak<mpsc::Sender<Message<M>>>)
where
    M: ManageConnection,
{
    tokio::spawn(async move {
        loop {
            let _ = interval.tick().await;
            if let Some(channel) = weak_shared.upgrade() {
                PoolInner { channel }.reap();
            } else {
                break;
            }
        }
    });
}
