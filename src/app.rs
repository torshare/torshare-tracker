use crate::servers;
use tokio::task::JoinHandle;

pub async fn start() -> Vec<JoinHandle<()>> {
    let mut jobs: Vec<JoinHandle<()>> = Vec::new();

    let http_server = tokio::spawn(async move {
        servers::http::HttpServer::start().await;
    });

    jobs.push(http_server);

    return jobs;
}
