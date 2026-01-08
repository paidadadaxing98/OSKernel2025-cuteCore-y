//! # 信号标志（SignalFlags）模块
//!
//! ## Overview
//! 本模块定义了内核中用于描述 **进程/任务异常与中断事件** 的信号标志集合，
//! 采用位标志（bitflags）的形式表示可同时存在的多种信号状态。
//!
//! `SignalFlags` 通常用于：
//! - 记录任务在执行过程中发生的异常（非法指令、算术异常、段错误等）
//! - 在任务退出或被内核终止时，向上层返回符合约定的错误码与错误信息
//!
//! ## Assumptions
//! - 信号仅由内核在受控路径中设置
//! - 同一时刻可能存在多个信号，但错误检查存在优先级
//! - 错误码采用类 Unix 约定（负数表示异常退出）
//!
//! ## Safety
//! - 本模块不涉及并发可变状态，仅进行位检查
//! - `bitflags` 宏生成的代码是内存安全的
//!
//! ## Invariants
//! - 每一种信号对应唯一的 bit 位
//! - `SignalFlags` 的值始终是若干合法信号的组合
//!
//! ## Behavior
//! - `check_error`：
//!   - 按固定优先级检查信号集合
//!   - 返回第一个匹配的错误码与描述字符串

use bitflags::*;

bitflags! {
    /// 信号标志集合
    ///
    /// ## Overview
    /// 使用位标志表示任务可能收到的信号，
    /// 支持高效组合与快速检查。
    ///
    /// ## Fields
    /// - `SIGINT`：
    ///   - 中断信号（通常由用户或外部事件触发）
    /// - `SIGILL`：
    ///   - 非法指令异常
    /// - `SIGABRT`：
    ///   - 程序异常终止
    /// - `SIGFPE`：
    ///   - 算术错误（如除零）
    /// - `SIGSEGV`：
    ///   - 段错误（非法内存访问）
    pub struct SignalFlags: u32 {
        const SIGINT    = 1 << 1;
        const SIGILL    = 1 << 3;
        const SIGABRT   = 1 << 5;
        const SIGFPE    = 1 << 7;
        const SIGSEGV   = 1 << 10;
        const SIGALRM	= 1 << 13;
        const SIGCHLD	= 1 << 16;
        const SIGVTALRM	= 1 << 25;
        const SIGPROF	= 1 << 26;
    }
}

impl SignalFlags {
    /// 检查当前信号集合中是否存在致命错误
    ///
    /// ## Overview
    /// 按预定义的优先级顺序检查信号标志，
    /// 若发现致命信号，则返回对应的错误码与说明信息。
    ///
    /// ## Returns
    /// - `Some((code, message))`：
    ///   - `code`：进程退出码（负数，符合类 Unix 约定）
    ///   - `message`：静态错误描述字符串
    /// - `None`：
    ///   - 当前不存在致命信号
    ///
    /// ## Invariants
    /// - 同一时间仅返回一个错误
    /// - 返回的错误码与信号类型一一对应
    ///
    /// ## Behavior
    /// - 检查顺序即信号处理优先级：
    ///     1. SIGINT
    ///     2. SIGILL
    ///     3. SIGABRT
    ///     4. SIGFPE
    ///     5. SIGSEGV
    pub fn check_error(&self) -> Option<(i32, &'static str)> {
        if self.contains(Self::SIGINT) {
            Some((-2, "Killed, SIGINT=2"))
        } else if self.contains(Self::SIGILL) {
            Some((-4, "Illegal Instruction, SIGILL=4"))
        } else if self.contains(Self::SIGABRT) {
            Some((-6, "Aborted, SIGABRT=6"))
        } else if self.contains(Self::SIGFPE) {
            Some((-8, "Erroneous Arithmetic Operation, SIGFPE=8"))
        } else if self.contains(Self::SIGSEGV) {
            Some((-11, "Segmentation Fault, SIGSEGV=11"))
        } else {
            None
        }
    }
    const EMPTY: SignalFlags = SignalFlags::empty();
    pub fn from_signum(signum: usize) -> Result<SignalFlags, ()> {
        match signum {
            0 => Ok(SignalFlags::EMPTY),
            1..=64 => Ok(SignalFlags::from_bits_truncate(1 << (signum - 1))),
            _ => Err(()),
        }
    }
}
