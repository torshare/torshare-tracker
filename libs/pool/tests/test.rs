use async_trait::async_trait;
use std::{
    error, fmt,
    marker::PhantomData,
    sync::atomic::{AtomicBool, Ordering},
};

use ts_pool::{ManageConnection, Pool};

#[derive(Debug, Default)]
struct FakeConnection;

struct OkManager<C> {
    _c: PhantomData<C>,
}

impl<C> OkManager<C> {
    fn new() -> Self {
        OkManager { _c: PhantomData }
    }
}

#[async_trait]
impl<C> ManageConnection for OkManager<C>
where
    C: Default + Send + Sync + 'static,
{
    type Connection = C;
    type Error = Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        Ok(Default::default())
    }

    async fn is_valid(&self, _conn: &mut Self::Connection) -> Result<(), Self::Error> {
        Ok(())
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        false
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Error;

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str("testerror")
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        "Error"
    }
}

#[tokio::test]
async fn test_get_connection() {
    let manager = OkManager::<FakeConnection>::new();

    let pool = ts_pool::Pool::builder().max_size(2).build(manager).unwrap();
    let conn = pool.get().await.unwrap();

    assert!(conn.is_some());
}

#[tokio::test]
async fn test_drop_on_broken() {
    static DROPPED: AtomicBool = AtomicBool::new(false);

    #[derive(Default)]
    struct Connection;

    impl Drop for Connection {
        fn drop(&mut self) {
            DROPPED.store(true, Ordering::SeqCst);
        }
    }

    struct Handler;

    #[async_trait]
    impl ManageConnection for Handler {
        type Connection = Connection;
        type Error = Error;

        async fn connect(&self) -> Result<Self::Connection, Self::Error> {
            Ok(Default::default())
        }

        async fn is_valid(&self, _conn: &mut Self::Connection) -> Result<(), Self::Error> {
            Ok(())
        }

        fn has_broken(&self, _: &mut Self::Connection) -> bool {
            true
        }
    }

    let pool = Pool::builder().build(Handler).unwrap();
    {
        let _ = pool.get().await.unwrap();
        tokio::task::yield_now().await;
    }

    assert!(DROPPED.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_max_size() {
    let manager = OkManager::<FakeConnection>::new();
    let pool = Pool::builder()
        .max_size(4)
        .min_idle(2)
        .build(manager)
        .unwrap();

    let mut vec = Vec::with_capacity(4);
    for _i in 0..10 {
        let conn = pool.get().await.unwrap();
        vec.push(conn.unwrap());

        if vec.len() == 4 {
            vec.pop();
        }
    }

    drop(vec);
    let state = pool.state().await.unwrap();

    assert_eq!(state.connections, 4);
    assert_eq!(state.idle_connections, 0);
}

#[tokio::test]
async fn test_drop_on_invalid() {
    #[derive(Default)]
    struct Connection;
    struct Handler;

    #[async_trait]
    impl ManageConnection for Handler {
        type Connection = Connection;
        type Error = Error;

        async fn connect(&self) -> Result<Self::Connection, Self::Error> {
            Ok(Default::default())
        }

        async fn is_valid(&self, _conn: &mut Self::Connection) -> Result<(), Self::Error> {
            Err(Error)
        }

        fn has_broken(&self, _: &mut Self::Connection) -> bool {
            false
        }
    }

    let manager = Handler;
    let pool = Pool::builder().max_size(10).build(manager).unwrap();

    {
        let _ = pool.get().await.unwrap().unwrap();
        tokio::task::yield_now().await;
    }

    {
        let _ = pool.get().await.unwrap().unwrap();
        tokio::task::yield_now().await;
    }

    let state = pool.state().await.unwrap();

    assert_eq!(state.connections, 1);
}
