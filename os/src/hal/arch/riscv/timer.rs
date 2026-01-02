//! 时钟与定时器模块
//! # Overview
//! 本模块提供内核时钟和定时器功能，封装对 RISC-V `time` 寄存器的访问
//! 以及通过 SBI 设置下一次定时器中断。
//!
//! # Design
//! - 使用 `time::read()` 获取当前时间戳（CPU 时钟 tick）
//! - 使用 SBI `set_timer` 设置下一次定时器触发时间
//! - 定时器频率由 `TICKS_PER_SEC` 控制，支持周期性触发
//! - 提供获取系统时钟频率接口 `get_clock_freq()`
//!
//! # Assumptions
//! - `CLOCK_FREQ` 为 CPU 时钟频率，单位 Hz
//! - SBI `set_timer` 能正确触发定时器中断
//! - 定时器中断处理函数能够及时响应触发
//!
//! # Safety
//! - 调用 `set_timer` 前应确保上下文允许 SBI 调用
//! - 时间读写基于 64 位寄存器，调用时需注意溢出
//!
//! # Invariants
//! - `TICKS_PER_SEC` 恒定为 25，定时器周期固定
//! - `set_next_trigger` 始终设置下一次触发在未来时间
//! - `get_time()` 返回单调递增时间戳

use super::sbi::set_timer;
use crate::hal::CLOCK_FREQ;
use riscv::register::time;

/// 每秒的定时器 tick 数
pub const TICKS_PER_SEC: usize = 25;

/// 获取当前时间（tick 数）
///
/// # Returns
/// - 当前 CPU tick 数
pub fn get_time() -> usize {
    time::read()
}

/// 设置下一次定时器触发
///
/// # Behavior
/// - 通过 SBI `set_timer` 设置下一次触发时间
/// - 触发时间 = 当前时间 + CLOCK_FREQ / TICKS_PER_SEC
pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}

/// 获取系统时钟频率
///
/// # Returns
/// - 系统时钟频率（Hz）
pub fn get_clock_freq() -> usize {
    CLOCK_FREQ
}
