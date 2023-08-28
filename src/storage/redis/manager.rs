use super::Result;
use crate::storage::Error;
use async_trait::async_trait;
use redis::{Client, IntoConnectionInfo, RedisError};
use ts_pool::{ManageConnection, Pool, PoolError, PooledConnection};

#[derive(Clone, Debug)]
pub struct RedisConnectionManager {
    client: Client,
}

impl RedisConnectionManager {
    pub fn new<T: IntoConnectionInfo>(params: T) -> Self {
        let client = Client::open(params).expect("Invalid connection URL");
        Self { client }
    }
}

#[async_trait]
impl ManageConnection for RedisConnectionManager {
    type Connection = redis::aio::Connection;
    type Error = RedisError;

    async fn connect(&self) -> std::result::Result<Self::Connection, Self::Error> {
        self.client.get_tokio_connection().await
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> std::result::Result<(), Self::Error> {
        let pong: String = redis::cmd("PING").query_async(conn).await?;
        match pong.as_str() {
            "PONG" => Ok(()),
            _ => Err((redis::ErrorKind::ResponseError, "ping request").into()),
        }
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        false
    }
}

pub async fn get_connection(
    pool: &Pool<RedisConnectionManager>,
) -> Result<PooledConnection<'_, RedisConnectionManager>> {
    match pool.get().await {
        Ok(Some(conn)) => Ok(conn),
        Ok(None) => Err("failed to get redis connection".into()),
        Err(err) => Err(err.into()),
    }
}

impl From<RedisError> for Error {
    fn from(err: RedisError) -> Self {
        Self::runtime(Box::new(err))
    }
}

impl From<PoolError> for Error {
    fn from(err: PoolError) -> Self {
        Self::runtime(Box::new(err))
    }
}
