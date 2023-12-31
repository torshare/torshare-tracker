use log::{info, LevelFilter};
use std::str::FromStr;
use ts_tracker::{app, config::TSConfig, signals::global_shutdown_signal};

#[cfg(feature = "memalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(not(target_env = "msvc"))]
#[cfg(not(feature = "memalloc"))]
#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[tokio::main]
async fn main() {
    let config = TSConfig::new().expect("failed to load config");
    setup_logger(&config.log_level);

    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    let jobs = app::start(config, stop_rx);

    tokio::select! {
        _ = global_shutdown_signal() => {
            info!("Shutting down tracker...");
            stop_tx.send(true).expect("failed to send shutdown signal");

            // Await for all jobs to shutdown
            futures::future::join_all(jobs).await;

            info!("Tracker shutdown complete");
        }
    }
}

fn setup_logger(log_level: &str) {
    let log_level = LevelFilter::from_str(log_level).unwrap();

    env_logger::builder()
        .filter_level(log_level)
        .format_timestamp(None)
        // .filter_module("ts_tracker", log_level)
        // .filter_module("ts_utils", log_level)
        .init();
}
