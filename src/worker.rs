use log::info;
use std::error::Error as StdError;
use std::fmt;
use tokio::sync::{mpsc, oneshot};

use crate::models::tracker::{AnnounceRequest, AnnounceResponse};

pub enum WorkerTask {
    Announce(AnnounceRequest),
    Scrape,
    Shutdown,
}

pub enum WorkerResponse {
    Announce(AnnounceResponse),
    Scrape,
    None,
}

pub type WorkerMessage = (WorkerTask, oneshot::Sender<WorkerResponse>);

pub struct Worker {
    // Define a channel to send messages to the Worker.
    sender: mpsc::Sender<WorkerMessage>,
}

impl Worker {
    pub fn new() -> Worker {
        let (sender, receiver) = mpsc::channel::<WorkerMessage>(1024);
        let mut work_handler = WorkHandler { receiver };
        tokio::spawn(async move { work_handler.run().await });

        Self { sender }
    }

    pub async fn work(&self, task: WorkerTask) -> Result<WorkerResponse, WorkerError> {
        let (sender, receiver) = oneshot::channel::<WorkerResponse>();
        self.sender.send((task, sender)).await?;
        receiver.await.map_err(|err| err.into())
    }

    pub async fn stop(&self) {
        let _ = self.sender.send((WorkerTask::Shutdown, oneshot::channel().0)).await;
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        info!("Dropping Worker");
        let _ = self.sender.try_send((WorkerTask::Shutdown, oneshot::channel().0));
    }
}

struct WorkHandler {
    /// Define a channel to receive messages from the Worker.
    receiver: mpsc::Receiver<WorkerMessage>,
}

impl WorkHandler {
    async fn run(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            let (task, sender) = msg;
            let _ = match task {
                WorkerTask::Announce(_) => {
                    let response = AnnounceResponse::default();
                    sender.send(WorkerResponse::Announce(response))
                }
                WorkerTask::Scrape => sender.send(WorkerResponse::Scrape),
                WorkerTask::Shutdown => {
                    self.receiver.close();
                    sender.send(WorkerResponse::None)
                }
            };
        }
    }
}

#[derive(Debug)]
pub struct WorkerError {
    inner: Option<Box<dyn StdError + Send + Sync>>,
}

impl From<mpsc::error::SendError<WorkerMessage>> for WorkerError {
    fn from(err: mpsc::error::SendError<WorkerMessage>) -> Self {
        Self {
            inner: Some(Box::new(err)),
        }
    }
}

impl From<oneshot::error::RecvError> for WorkerError {
    fn from(err: oneshot::error::RecvError) -> Self {
        Self {
            inner: Some(Box::new(err)),
        }
    }
}

impl fmt::Display for WorkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.inner {
            Some(inner) => inner.fmt(f),
            None => write!(f, "unknown error"),
        }
    }
}

impl StdError for WorkerError {}

pub type WorkerResult = std::result::Result<WorkerResponse, WorkerError>;
