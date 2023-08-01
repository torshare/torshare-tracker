use std::sync::Arc;

use crate::{servers::http::HttpServer, worker::Worker};
use tokio::task::JoinHandle;

pub async fn start() -> Vec<JoinHandle<()>> {
    let mut jobs: Vec<JoinHandle<()>> = Vec::new();
    let worker = Arc::new(Worker::new());

    let worker_clone = worker.clone();
    let http_server_job = tokio::spawn(async move {
        let http_server = HttpServer::new(worker_clone);
        http_server.start().await;
    });

    // jobs.push(worker_job);
    jobs.push(http_server_job);

    return jobs;
}
