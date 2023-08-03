use log::info;
use torshare_tracker::app;

#[tokio::main]
async fn main() {
    env_logger::init();
    let jobs = app::start().await;

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Shutting down tracker...");

            // Await for all jobs to shutdown
            futures::future::join_all(jobs).await;

            info!("Tracker shutdown complete");
        }
    }
}
