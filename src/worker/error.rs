use std::error::Error as StdError;
use std::fmt;

use super::TaskPacket;
use tokio::sync::{mpsc, oneshot};

type Cause = Box<dyn StdError + Send + Sync>;

pub struct Error {
    inner: Box<ErrorImpl>,
}

struct ErrorImpl {
    kind: Kind,
    cause: Option<Cause>,
}

#[allow(unused)]
#[derive(Debug)]
enum Kind {
    Send,
    Recv,
    TooManyRequests,
    Custom(String),
}

impl From<mpsc::error::SendError<TaskPacket>> for Error {
    fn from(err: mpsc::error::SendError<TaskPacket>) -> Self {
        Self {
            inner: Box::new(ErrorImpl {
                kind: Kind::Send,
                cause: Some(Box::new(err)),
            }),
        }
    }
}

impl From<oneshot::error::RecvError> for Error {
    fn from(err: oneshot::error::RecvError) -> Self {
        Self {
            inner: Box::new(ErrorImpl {
                kind: Kind::Recv,
                cause: Some(Box::new(err)),
            }),
        }
    }
}

impl From<&str> for Error {
    fn from(err: &str) -> Self {
        Self {
            inner: Box::new(ErrorImpl {
                kind: Kind::Custom(err.to_string()),
                cause: None,
            }),
        }
    }
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Self {
            inner: Box::new(ErrorImpl {
                kind: Kind::Custom(err),
                cause: None,
            }),
        }
    }
}

impl Error {
    /// The error's standalone message, without the message from the source.
    pub fn message(&self) -> impl fmt::Display + '_ {
        self.description()
    }

    fn description(&self) -> &str {
        match self.inner.kind {
            Kind::Send => "failed to send message to task handler",
            Kind::Recv => "failed to receive message from worker",
            Kind::TooManyRequests => "too many requests",
            Kind::Custom(ref str) => str,
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_tuple("worker::Error");
        f.field(&self.inner.kind);
        if let Some(ref cause) = self.inner.cause {
            f.field(cause);
        }
        f.finish()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.description())
    }
}

impl StdError for Error {}

/// Define a type alias for the return type of a worker task.
pub type Result<T> = std::result::Result<T, Error>;
