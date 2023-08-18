use std::time::{SystemTime, UNIX_EPOCH};

/// The `Loggable` trait represents an interface for objects that can be logged.
pub trait Loggable {
    /// Logs information about the object and returns a formatted log message as a String.
    /// This method must be implemented by types that implement the `Loggable` trait.
    ///
    /// # Returns
    ///
    /// The log message as a String containing the information about the object.
    fn log(&self) -> String;
}

pub fn now() -> u64 {
    let start = SystemTime::now();
    start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}
