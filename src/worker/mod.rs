mod error;
pub use error::{Error, Result};
mod tasks;
pub use tasks::full_scrape::FullScrapeProcessor;

use self::tasks::{announce, full_scrape, scrape, State, TaskExecutor};
use crate::{config::TSConfig, storage::create_new_storage};
use log::{debug, info};
use std::sync::Arc;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

pub struct Worker {
    sender: mpsc::Sender<TaskPacket>,
    receiver: Option<mpsc::Receiver<TaskPacket>>,
    state: Option<State>,
}

const WORKER_POOL_SIZE: usize = 10_000;

impl Worker {
    /// Create a new `Worker`.
    pub fn new(config: Arc<TSConfig>) -> Worker {
        let storage = create_new_storage(config.clone())
            .expect("Failed to create storage")
            .into();

        let (sender, receiver) = mpsc::channel::<TaskPacket>(WORKER_POOL_SIZE);
        let state = State { storage, config };
        Self {
            sender,
            receiver: Some(receiver),
            state: Some(state),
        }
    }

    /// Start the `WorkerLoop` to handle incoming tasks.
    pub fn start(&mut self) -> JoinHandle<()> {
        let receiver = self.receiver.take().expect("Worker loop already started");
        let state = self.state.take().unwrap();

        tokio::spawn(async move {
            let mut worker_loop = WorkerLoop { receiver, state };
            worker_loop.run().await
        })
    }

    /// Send a task to the `Worker` for execution.
    pub async fn work(&self, task: Task) -> Result<TaskOutput> {
        let (sender, receiver) = oneshot::channel::<Result<TaskOutput>>();
        self.sender.send((task, sender)).await?;
        receiver.await?
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let sender = self.sender.clone();
        tokio::spawn(async move {
            let _ = sender.send((Task::Shutdown, oneshot::channel().0)).await;
        });
    }
}

pub enum Task {
    Announce(announce::Input),
    Scrape(scrape::Input),
    FullScrape(full_scrape::Input),
    UpdateState(State),
    Shutdown,
}

pub enum TaskOutput {
    Announce(announce::Output),
    Scrape(scrape::Output),
    FullScrape(full_scrape::Output),
    None,
}

type TaskSender = oneshot::Sender<Result<TaskOutput>>;
type TaskPacket = (Task, TaskSender);

struct WorkerLoop {
    receiver: mpsc::Receiver<TaskPacket>,
    state: State,
}

impl WorkerLoop {
    pub(super) async fn run(&mut self) {
        let executor = Executor;
        info!("Worker loop started");

        while let Some(msg) = self.receiver.recv().await {
            let (task, sender) = msg;
            debug!("Worker loop received task {:?}", task);

            match task {
                Task::Announce(input) => {
                    executor.execute(announce::TaskExecutor, input, sender, self.state.clone())
                }

                Task::Scrape(input) => {
                    executor.execute(scrape::TaskExecutor, input, sender, self.state.clone())
                }

                Task::FullScrape(input) => {
                    executor.execute(full_scrape::TaskExecutor, input, sender, self.state.clone())
                }

                Task::UpdateState(state) => {
                    self.state = state;
                    let _ = sender.send(Ok(TaskOutput::None));
                }
                Task::Shutdown => {
                    self.receiver.close();
                    let _ = sender.send(Ok(TaskOutput::None));
                }
            };
        }

        info!("Worker loop stopped");
    }
}

struct Executor;

impl Executor {
    fn execute<E, I, O>(&self, executor: E, input: I, sender: TaskSender, state: State)
    where
        E: TaskExecutor<Input = I, Output = O> + 'static,
        I: Send + 'static,
    {
        tokio::spawn(async move {
            let response = executor.execute(input, state).await;
            let _ = sender.send(response);
        });
    }
}

impl std::fmt::Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Task::Announce(_) => write!(f, "Announce"),
            Task::Scrape(_) => write!(f, "Scrape"),
            Task::FullScrape(_) => write!(f, "FullScrape"),
            Task::UpdateState(_) => write!(f, "UpdateState"),
            Task::Shutdown => write!(f, "Shutdown"),
        }
    }
}
