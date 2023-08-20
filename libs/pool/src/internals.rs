use tokio::sync::{mpsc, oneshot, OwnedSemaphorePermit, Semaphore};

use crate::{api::Builder, ManageConnection};
use std::{
    collections::VecDeque,
    sync::{Arc, Weak},
    time::Instant,
};

/// The guts of a `Pool`.
#[allow(missing_debug_implementations)]
pub(crate) struct SharedPool<M>
where
    M: ManageConnection + Send,
{
    shared: Arc<Shared<M>>,
    internals: PoolInternals<M>,
}

impl<M> SharedPool<M>
where
    M: ManageConnection + Send,
{
    pub(crate) fn channel(manager: M, builder: Builder<M>) -> Arc<mpsc::Sender<Message<M>>> {
        let (tx, rx) = mpsc::channel(builder.max_size as usize * 2);
        let tx = Arc::new(tx);
        let weak_tx = Arc::downgrade(&tx);

        tokio::spawn(async move {
            let shared = Shared::new(manager, builder, weak_tx);
            let pool = Self {
                shared: Arc::new(shared),
                internals: PoolInternals::default(),
            };

            let _ = pool.run(rx).await;
        });

        return tx;
    }

    fn state(&self) -> State {
        let shared = &self.shared;
        let conns = &self.internals.conns;

        State {
            connections: shared.statics.max_size - shared.semaphore.available_permits() as u32,
            idle_connections: conns.len() as u32,
        }
    }

    async fn send_connection(&mut self, tx: GetConnTx<M::Connection, M::Error>) {
        loop {
            match self.internals.get() {
                Some(conn) => {
                    let shared = self.shared.clone();
                    tokio::spawn(async move { shared.send_connection(conn.into(), tx).await });
                    return;
                }
                None if self.shared.semaphore.available_permits() > 0 => {
                    let shared = self.shared.clone();
                    tokio::spawn(async move { shared.make_new_connection(tx).await });
                    return;
                }
                _ => {}
            };

            tokio::task::yield_now().await;
        }
    }

    async fn run(mut self: SharedPool<M>, mut rx: mpsc::Receiver<Message<M>>) {
        while let Some(msg) = rx.recv().await {
            match msg {
                Message::GetConn(tx) => {
                    self.send_connection(tx).await;
                }

                Message::PutConn(mut conn) => {
                    if !self.shared.manager.has_broken(&mut conn.conn) {
                        self.internals.put_back(conn);
                    }
                }

                Message::State(tx) => {
                    let _ = tx.send(self.state());
                }

                Message::Reap => {
                    let idle_timeout = self.shared.statics.idle_timeout.unwrap_or_default();
                    let mut conns = Vec::new();
                    let now = Instant::now();

                    while let Some(conn) = self.internals.get() {
                        if now - conn.idle_start < idle_timeout {
                            conns.push(conn);
                        }
                    }

                    self.internals.conns = conns.into();
                }
            };
        }
    }
}

type GetConnTx<C, E> = oneshot::Sender<Result<Option<Conn<C>>, E>>;

pub(crate) enum Message<M>
where
    M: ManageConnection + Send,
{
    GetConn(GetConnTx<M::Connection, M::Error>),
    PutConn(Conn<M::Connection>),
    State(oneshot::Sender<State>),
    Reap,
}

/// The pool data that must be protected by a lock.
#[allow(missing_debug_implementations)]
pub(crate) struct PoolInternals<M>
where
    M: ManageConnection,
{
    conns: VecDeque<IdleConn<M::Connection>>,
}

impl<M> PoolInternals<M>
where
    M: ManageConnection,
{
    fn put_back(&mut self, conn: Conn<M::Connection>) {
        self.conns.push_back(conn.into());
    }

    fn get(&mut self) -> Option<IdleConn<M::Connection>> {
        self.conns.pop_front()
    }
}

impl<M> Default for PoolInternals<M>
where
    M: ManageConnection,
{
    fn default() -> Self {
        Self {
            conns: VecDeque::new(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Conn<C>
where
    C: Send,
{
    pub(crate) conn: C,
    _permit: OwnedSemaphorePermit,
}

impl<C: Send> Conn<C> {
    pub(crate) fn new(conn: C, permit: OwnedSemaphorePermit) -> Self {
        Self {
            conn,
            _permit: permit,
        }
    }
}

impl<C: Send> From<IdleConn<C>> for Conn<C> {
    fn from(conn: IdleConn<C>) -> Self {
        conn.conn
    }
}

struct IdleConn<C>
where
    C: Send,
{
    conn: Conn<C>,
    idle_start: Instant,
}

impl<C: Send> From<Conn<C>> for IdleConn<C> {
    fn from(conn: Conn<C>) -> Self {
        IdleConn {
            conn,
            idle_start: Instant::now(),
        }
    }
}

/// Information about the state of a `Pool`.
#[derive(Debug)]
#[non_exhaustive]
pub struct State {
    /// The number of connections currently being managed by the pool.
    pub connections: u32,

    /// The number of idle connections.
    pub idle_connections: u32,
}

struct Shared<M>
where
    M: ManageConnection + Send,
{
    manager: M,
    statics: Builder<M>,
    semaphore: Arc<Semaphore>,
    weak_tx: Weak<mpsc::Sender<Message<M>>>,
}

impl<M> Shared<M>
where
    M: ManageConnection + Send,
{
    fn new(manager: M, builder: Builder<M>, weak_tx: Weak<mpsc::Sender<Message<M>>>) -> Self {
        Self {
            manager,
            semaphore: Arc::new(Semaphore::new(builder.max_size as usize)),
            statics: builder,
            weak_tx,
        }
    }

    async fn make_new_connection(&self, tx: GetConnTx<M::Connection, M::Error>) {
        if self.semaphore.available_permits() == 0 {
            return;
        }

        let permit = self.semaphore.clone().acquire_owned().await.unwrap();
        let result = match self.manager.connect().await {
            Ok(conn) => Some(Conn::new(conn, permit)),
            Err(err) => {
                self.statics.error_sink.sink(err);
                None
            }
        };

        if let Some(conn) = result {
            let _ = tx.send(Ok(Some(conn)));
        }
    }

    async fn send_connection(
        &self,
        mut conn: Conn<M::Connection>,
        tx: GetConnTx<M::Connection, M::Error>,
    ) {
        if self.statics.test_on_check_out {
            if let Err(err) = self.manager.is_valid(&mut conn.conn).await {
                self.statics.error_sink.sink(err);

                if let Some(channel) = self.weak_tx.upgrade() {
                    let _ = channel.send(Message::GetConn(tx)).await;
                }

                return;
            }
        }

        let _ = tx.send(Ok(Some(conn)));
    }
}