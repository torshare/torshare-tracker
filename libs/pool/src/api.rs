use async_trait::async_trait;

use crate::{
    inner::PoolInner,
    internals::{Conn, State},
};
use std::{
    borrow::Cow,
    fmt::{self, Debug},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    time::Duration,
};

/// A generic connection pool.
pub struct Pool<M>
where
    M: ManageConnection,
{
    pub(crate) inner: PoolInner<M>,
}

impl<M: ManageConnection> Pool<M> {
    /// Returns a `Builder` instance to configure a new pool.
    pub fn builder() -> Builder<M> {
        Builder::new()
    }

    /// Returns a new `PooledConnection`.
    pub async fn get(&self) -> Result<Option<PooledConnection<'_, M>>, PoolError<M::Error>> {
        self.inner
            .get()
            .await
            .map(|conn| conn.map(|conn| PooledConnection::new(&self.inner, conn)))
    }

    pub async fn state(&self) -> Option<State> {
        self.inner.state().await
    }
}

/// A builder for a connection pool.
#[derive(Debug)]
pub struct Builder<M: ManageConnection> {
    /// The maximum number of connections allowed.
    pub(crate) max_size: u32,

    /// The minimum idle connection count the pool will attempt to maintain.
    pub(crate) min_idle: Option<u32>,

    /// Whether or not to test the connection on checkout.
    pub(crate) test_on_check_out: bool,

    /// The duration, if any, after which idle_connections in excess of `min_idle` are closed.
    pub(crate) idle_timeout: Option<Duration>,

    /// The duration to wait to start a connection before giving up.
    pub(crate) connection_timeout: Duration,

    /// The time interval used to wake up and reap connections.
    pub(crate) reaper_rate: Duration,

    /// Enable/disable automatic retries on connection creation.
    pub(crate) retry_connection: bool,

    /// The error sink.
    pub(crate) error_sink: Box<dyn ErrorSink<M::Error>>,

    _p: PhantomData<M>,
}

impl<M: ManageConnection> Default for Builder<M> {
    fn default() -> Self {
        Self {
            max_size: 10,
            min_idle: None,
            test_on_check_out: true,
            idle_timeout: Some(Duration::from_secs(120)),
            reaper_rate: Duration::from_secs(30),
            connection_timeout: Duration::from_secs(15),
            error_sink: Box::new(NopErrorSink),
            retry_connection: true,
            _p: PhantomData,
        }
    }
}

impl<M: ManageConnection> Builder<M> {
    /// Constructs a new `Builder`.
    ///
    /// Parameters are initialized with their default values.
    #[must_use]
    pub fn new() -> Self {
        Builder::default()
    }

    /// Sets the maximum number of connections managed by the pool.
    /// Defaults to 10.
    #[must_use]
    pub fn max_size(mut self, max_size: u32) -> Self {
        assert!(max_size > 0, "max_size must be greater than zero!");
        self.max_size = max_size;
        self
    }

    /// Sets the minimum idle connection count maintained by the pool.
    /// Defaults to None.
    #[must_use]
    pub fn min_idle(mut self, min_idle: u32) -> Self {
        self.min_idle = Some(min_idle);
        self
    }

    /// Set the sink for errors that occur on background tasks.
    /// This can be used to log and monitor failures.
    ///
    /// Defaults to `NopErrorSink`.
    #[must_use]
    pub fn error_sink(mut self, error_sink: Box<dyn ErrorSink<M::Error>>) -> Self {
        self.error_sink = error_sink;
        self
    }

    /// Sets the duration after which idle connections in excess of `min_idle` are closed.
    /// Defaults to 60 seconds.
    #[must_use]
    pub fn idle_timeout(mut self, idle_timeout: Duration) -> Self {
        self.idle_timeout = Some(idle_timeout);
        self
    }

    /// Sets the time interval used to wake up and reap connections.
    /// Defaults to 30 seconds.
    #[must_use]
    pub fn reaper_rate(mut self, reaper_rate: Duration) -> Self {
        self.reaper_rate = reaper_rate;
        self
    }

    /// Instructs the pool to automatically retry connection creation if it fails.
    /// Defaults to true.
    #[must_use]
    pub fn retry_connection(mut self, retry_connection: bool) -> Self {
        self.retry_connection = retry_connection;
        self
    }

    /// Sets the duration to wait to start a connection before giving up.
    /// Defaults to 15 seconds.
    #[must_use]
    pub fn connection_timeout(mut self, connection_timeout: Duration) -> Self {
        self.connection_timeout = connection_timeout;
        self
    }

    /// Sets whether or not to test the connection on checkout.
    /// Defaults to true.
    #[must_use]
    pub fn test_on_check_out(mut self, test_on_check_out: bool) -> Self {
        self.test_on_check_out = test_on_check_out;
        self
    }

    fn build_inner(self, manager: M) -> Pool<M> {
        if let Some(min_idle) = self.min_idle {
            assert!(
                self.max_size >= min_idle,
                "min_idle must be no larger than max_size"
            );
        }

        Pool {
            inner: PoolInner::new(self, manager),
        }
    }

    pub fn build(self, manager: M) -> Result<Pool<M>, M::Error> {
        let pool = self.build_inner(manager);
        Ok(pool)
    }
}

/// A trait which provides connection-specific functionality.
#[async_trait]
pub trait ManageConnection: Sized + Send + Sync + 'static {
    /// The connection type this manager deals with.
    type Connection: Send + 'static;

    /// The error type returned by `Connection`s.
    type Error: fmt::Debug + Send + 'static;

    /// Attempts to create a new connection.
    async fn connect(&self) -> Result<Self::Connection, Self::Error>;

    /// Determines if the connection is still connected to the database.
    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error>;

    /// Determines if the connection should be discarded.
    ///
    /// This will be called synchronously every time a connection is returned to the pool, so it should not block.
    /// If it returns true, the connection will be discarded.
    fn has_broken(&self, conn: &mut Self::Connection) -> bool;
}

pub struct PooledConnection<'a, M>
where
    M: ManageConnection,
{
    pool: Cow<'a, PoolInner<M>>,
    conn: Option<Conn<M::Connection>>,
}

impl<'a, M> PooledConnection<'a, M>
where
    M: ManageConnection,
{
    pub(crate) fn new(pool: &'a PoolInner<M>, conn: Conn<M::Connection>) -> Self {
        Self {
            pool: Cow::Borrowed(pool),
            conn: Some(conn),
        }
    }
}

impl<'a, M> Deref for PooledConnection<'a, M>
where
    M: ManageConnection,
{
    type Target = M::Connection;
    fn deref(&self) -> &Self::Target {
        &self.conn.as_ref().unwrap().conn
    }
}

impl<'a, M> DerefMut for PooledConnection<'a, M>
where
    M: ManageConnection,
{
    fn deref_mut(&mut self) -> &mut M::Connection {
        &mut self.conn.as_mut().unwrap().conn
    }
}

impl<'a, M> fmt::Debug for PooledConnection<'a, M>
where
    M: ManageConnection,
    M::Connection: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.conn.as_ref().unwrap().conn, fmt)
    }
}

impl<'a, M> Drop for PooledConnection<'a, M>
where
    M: ManageConnection,
{
    fn drop(&mut self) {
        self.pool.as_ref().put_back(self.conn.take());
    }
}

impl<M> Clone for Pool<M>
where
    M: ManageConnection,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<M> fmt::Debug for Pool<M>
where
    M: ManageConnection,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("Pool({:?})", self.inner))
    }
}

/// Represents errors that can occur while working with a connection pool.
pub enum PoolError<E> {
    /// An error occurred while creating a new connection.
    ConnectionError(E),

    /// The pool has been closed.
    Closed,

    /// A timeout occurred while waiting for a connection.
    Timeout,
}

impl<E: Debug> From<E> for PoolError<E> {
    fn from(err: E) -> Self {
        Self::ConnectionError(err)
    }
}

impl<E: Debug> fmt::Display for PoolError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl<E: Debug> fmt::Debug for PoolError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionError(err) => write!(f, "{:?}", err),
            Self::Closed => write!(f, "Closed"),
            Self::Timeout => write!(f, "Timeout"),
        }
    }
}

/// A trait to receive errors generated by connection management that aren't
/// tied to any particular caller.
pub trait ErrorSink<E>: fmt::Debug + Send + Sync + 'static {
    /// Receive an error
    fn sink(&self, error: E);

    /// Clone this sink.
    fn boxed_clone(&self) -> Box<dyn ErrorSink<E>>;
}

/// An `ErrorSink` implementation that does nothing.
#[derive(Debug, Clone, Copy)]
pub struct NopErrorSink;

impl<E> ErrorSink<E> for NopErrorSink {
    fn sink(&self, _: E) {}

    fn boxed_clone(&self) -> Box<dyn ErrorSink<E>> {
        Box::new(*self)
    }
}
