//! SBI 调用模块
//! # Overview
//! 本模块提供对 RISC-V SBI（Supervisor Binary Interface）的封装，用于内核和平台交互。
//! 包含定时器设置、控制台输入输出、IPI（Inter-Processor Interrupt）、页表同步和系统关机等功能。
//!
//! # Design
//! - 所有 SBI 调用通过 `ecall` 指令触发陷入 S 模式执行。
//! - `sbi_call` 函数是通用封装，将函数号和参数传递给 SBI。
//! - 上层函数（如 `set_timer`、`console_putchar`）直接调用 `sbi_call`，简化内核接口。
//!
//! # Assumptions
//! - 内核运行在 S 模式下，并且底层固件或 SBI 实现可响应这些调用。
//! - 调用参数和返回值遵循 RISC-V SBI ABI 规范。
//!
//! # Safety
//! - `sbi_call` 使用裸 `asm!` 调用 ecall，需要确保传入参数正确且安全。
//! - 上层调用者应确保在允许上下文中使用，例如不能在中断关闭时调用可能阻塞的函数。
//!
//! # Invariants
//! - 每个 SBI 功能号唯一对应一个功能。
//! - `sbi_call` 返回值符合 SBI ABI 规范，未定义情况不会破坏系统状态。
//! - 控制台输出和输入不会改变内核栈或关键寄存器状态。

#![allow(unused)]

use core::arch::asm;

/// SBI (Supervisor Binary Interface) 系统调用常量
const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;
const SBI_CLEAR_IPI: usize = 3;
const SBI_SEND_IPI: usize = 4;
const SBI_REMOTE_FENCE_I: usize = 5;
const SBI_REMOTE_SFENCE_VMA: usize = 6;
const SBI_REMOTE_SFENCE_VMA_ASID: usize = 7;
const SBI_SHUTDOWN: usize = 8;

/// 通用 SBI 调用封装函数
///
/// # Fields
/// - `which`：SBI 功能号
/// - `arg0`、`arg1`、`arg2`：传递给 SBI 的三个参数
///
/// # Returns
/// - SBI 返回值
///
/// # Safety
/// - 直接使用裸汇编 `ecall` 指令，必须保证参数正确。
/// - 此函数可能陷入 S 模式执行，调用上下文需安全。
#[inline(always)]
/// `ecall` wrapper to switch trap into S level.
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe {
        asm!(
        "ecall",
        inlateout("x10") arg0 => ret,
        in("x11") arg1,
        in("x12") arg2,
        in("x17") which,
        );
    }
    ret
}

/// 设置定时器
///
/// # Arguments
/// - `timer`：定时器周期或目标时间（具体根据 SBI 平台实现）
///
/// 调用 SBI 的 `SBI_SET_TIMER` 功能，触发定时器中断。
pub fn set_timer(timer: usize) {
    sbi_call(SBI_SET_TIMER, timer, 0, 0);
}

/// 控制台输出一个字符
///
/// # Arguments
/// - `c`：要输出的字符（ASCII 值）
///
/// 调用 SBI 的 `SBI_CONSOLE_PUTCHAR` 功能，输出到串口或终端。
pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0);
}

/// 控制台读取一个字符
///
/// # Returns
/// - 读取的字符（ASCII 值），若无输入返回负值或 0（取决于 SBI 实现）
///
/// 调用 SBI 的 `SBI_CONSOLE_GETCHAR` 功能。
pub fn console_getchar() -> usize {
    sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0)
}

/// 刷新控制台缓冲区
///
/// 当前实现为空函数，SBI 不提供显式刷新接口。
pub fn console_flush() {}

/// 关机系统
///
/// 调用 SBI 的 `SBI_SHUTDOWN` 功能，尝试关机或重置系统。
///
/// # Panics
/// - 如果关机失败，会触发 panic。
pub fn shutdown() -> ! {
    println!("run shutdown");
    sbi_call(SBI_SHUTDOWN, 0, 0, 0);
    panic!("It should shutdown!");
}
