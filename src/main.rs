use log::info;
use torshare_tracker::app;

#[tokio::main]
async fn main() {
    let jobs = app::start().await;

    // handle the signals
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Tracker shutting down..");

            // Await for all jobs to shutdown
            futures::future::join_all(jobs).await;
            info!("Tracker successfully shutdown.");
        }
    }
}
