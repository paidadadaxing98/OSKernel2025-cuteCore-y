use crate::hal::{get_clock_freq, get_time};
use crate::sync::UPIntrFreeCell;
use crate::task::{wakeup_task, TaskControlBlock};
use alloc::collections::BinaryHeap;
use alloc::sync::Arc;
use core::cmp::Ordering;
use core::time::Duration;
use lazy_static::lazy_static;

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

pub struct TimerCondVar {
    pub expire_ms: usize,
    pub task: Arc<TaskControlBlock>,
}

impl PartialEq for TimerCondVar {
    fn eq(&self, other: &Self) -> bool {
        self.expire_ms == other.expire_ms
    }
}
impl Eq for TimerCondVar {}
impl PartialOrd for TimerCondVar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let a = -(self.expire_ms as isize);
        let b = -(other.expire_ms as isize);
        Some(a.cmp(&b))
    }
}

impl Ord for TimerCondVar {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

lazy_static! {
    static ref TIMERS: UPIntrFreeCell<BinaryHeap<TimerCondVar>> =
        unsafe { UPIntrFreeCell::new(BinaryHeap::<TimerCondVar>::new()) };
}

pub fn add_timer(expire_ms: usize, task: Arc<TaskControlBlock>) {
    let mut timers = TIMERS.exclusive_access();
    timers.push(TimerCondVar { expire_ms, task });
}

pub fn check_timer() {
    let current_ms = get_time_ms();
    TIMERS.exclusive_session(|timers| {
        while let Some(timer) = timers.peek() {
            if timer.expire_ms <= current_ms {
                wakeup_task(Arc::clone(&timer.task));
                timers.pop();
            } else {
                break;
            }
        }
    });
}
