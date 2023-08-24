use std::error::Error as StdError;
use std::fmt;

use super::TaskPacket;
use crate::storage;
use tokio::sync::{mpsc, oneshot};

type Cause = Box<dyn StdError + Send + Sync>;

/// An error that can occur while interacting with the worker.
pub struct Error {
    inner: Box<ErrorImpl>,
}

struct ErrorImpl {
    kind: Kind,
    cause: Option<Cause>,
}

#[derive(Debug)]
enum Kind {
    Send,
    Recv,
    Storage,
    Custom(&'static str),
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

impl From<&'static str> for Error {
    fn from(err: &'static str) -> Self {
        Self {
            inner: Box::new(ErrorImpl {
                kind: Kind::Custom(err),
                cause: None,
            }),
        }
    }
}

impl From<storage::Error> for Error {
    fn from(value: storage::Error) -> Self {
        Self {
            inner: Box::new(ErrorImpl {
                kind: Kind::Storage,
                cause: Some(Box::new(value)),
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
            Kind::Storage => "storage error",
            Kind::Custom(str) => str,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.description())
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.inner.cause.as_ref().map(|cause| &**cause as _)
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

/// Define a type alias for the return type of a worker task.
pub type Result<T> = std::result::Result<T, Error>;
