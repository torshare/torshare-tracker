use torshare_tracker::app;

#[tokio::main]
async fn main() {
    let jobs = app::start().await;

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("Shutting down tracker...");
            // Await for all jobs to shutdown
            futures::future::join_all(jobs).await;
            println!("Tracker shutdown complete");
        }
    }
}
