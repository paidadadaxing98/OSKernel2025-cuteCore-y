use core::time::Duration;
use crate::hal::{get_clock_freq, get_time};

pub const MSEC_PER_SEC: usize = 1000;

pub const USEC_PER_SEC: usize = 1_000_000;
pub const USEC_PER_MSEC: usize = 1_000;

pub const NSEC_PER_SEC: usize = 1_000_000_000;
pub const NSEC_PER_MSEC: usize = 1_000_000;
pub const NSEC_PER_USEC: usize = 1_000;


pub fn get_time_sec() -> usize {
    get_time() / get_clock_freq()
}


pub fn get_time_ms() -> usize {
    get_time() / (get_clock_freq() / MSEC_PER_SEC)
}


pub fn get_time_us() -> usize {
    get_time() / (get_clock_freq() / USEC_PER_SEC)
}

pub fn current_time_duration() -> Duration {
    Duration::from_micros(get_time_us() as u64)
}

/// Check timer and handle timer-related tasks.
pub fn check_timer() {
    todo!()
}




