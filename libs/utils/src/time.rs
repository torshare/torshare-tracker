#[cfg(target_has_atomic = "64")]
pub use coarsetime::{Clock, Duration, Instant};

#[cfg(not(target_has_atomic = "64"))]
pub use std::time::{Duration, Instant};

#[cfg(not(target_has_atomic = "64"))]
pub struct Clock;

#[cfg(not(target_has_atomic = "64"))]
impl Clock {
    pub fn now_since_epoch() -> Duration {
        match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(n) => n,
            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        }
    }

    pub fn recent_since_epoch() -> Duration {
        Clock::now_since_epoch()
    }

    pub fn update() {
        // Nothing to do here
    }
}
