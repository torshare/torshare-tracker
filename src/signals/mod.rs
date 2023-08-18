//! This module contains functions to handle signals.
use tokio::{signal, sync::watch};

/// Asynchronously waits for a global shutdown signal, either from Ctrl+C or the termination signal.
///
/// The `global_shutdown_signal` function sets up async tasks to await the reception of two types
/// of shutdown signals: Ctrl+C signal and the termination signal. The function blocks until
/// either of these signals is received. Once a signal is received, the function will return, and
/// the async task that called it will continue execution.
///
/// # Behavior on Unix-like Systems (e.g., Linux)
///
/// - On Unix-like systems, the function awaits the reception of both Ctrl+C signal and the
///   termination signal. When either signal is detected, the function gracefully terminates
///   and allows the async task to continue execution. If an error occurs while waiting for the
///   signals, the function will panic with an error message.
///
/// # Behavior on Non-Unix Platforms (e.g., Windows)
///
/// - On non-Unix platforms where the `signal::unix` module is not available (e.g., Windows),
///   the function sets up a pending future using `std::future::pending`. This allows the
///   function to await the termination signal on Unix platforms while effectively skipping the
///   termination signal handling on non-Unix platforms. On non-Unix platforms, the function
///   behaves as if the termination signal is never received, and the async task continues
///   execution without any explicit termination signal handling.
pub async fn global_shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {}
    }
}

pub type StopSignalRx = watch::Receiver<bool>;
