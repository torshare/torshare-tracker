use std::sync::Arc;

use crate::{
    config::TSConfig,
    servers::{HttpServer, State},
    signals::StopSignalRx,
    worker::Worker,
};
use tokio::task::JoinHandle;

pub fn start(config: TSConfig, stop_signal_rx: StopSignalRx) -> Vec<JoinHandle<()>> {
    let mut jobs: Vec<JoinHandle<()>> = Vec::new();
    let config = Arc::new(config);

    let mut worker = Worker::new(config.clone());
    let worker_job = start_worker(&mut worker);

    jobs.push(worker_job);

    let state = State::new(Arc::new(worker), config);
    let http_server_job = start_http_server(state, stop_signal_rx.clone());

    jobs.push(http_server_job);

    return jobs;
}

fn start_worker(worker: &mut Worker) -> JoinHandle<()> {
    worker.start()
}

fn start_http_server(state: State, stop_signal_recv: StopSignalRx) -> JoinHandle<()> {
    tokio::spawn(async move {
        let http_server = HttpServer::new(state);
        http_server
            .start(stop_signal_recv)
            .await
            .expect("Failed to start http server.");
    })
}
