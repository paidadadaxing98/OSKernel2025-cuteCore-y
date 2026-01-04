//! # 内核同步原语模块（sync）
//!
//! ## Overview
//! 本模块是内核中 **所有基础同步原语的统一入口**，
//! 对外以 `pub use` 的形式导出可供系统其他子模块
//! （如任务调度、系统调用、文件系统等）使用的同步设施。
//!
//! 模块内部按功能拆分为多个子模块：
//! - `mutex`：互斥锁抽象及其具体实现（自旋 / 阻塞）
//! - `semaphore`：计数型信号量
//! - `condvar`：条件变量
//! - `up`：单处理器环境下的内部可变性与中断屏蔽封装
//!
//! 该模块是内核并发控制的基础设施层，
//! 负责在 **单处理器 + 中断并发模型** 下提供安全、可组合的同步机制。
//!
//! ## Assumptions
//! - 系统运行在单处理器环境
//! - 不存在真正的多核并行，仅可能被中断或调度切换打断
//! - 所有同步原语都依赖 `UPIntrFreeCell` 提供的关中断互斥语义
//!
//! ## Safety
//! - 所有 `unsafe impl Sync` 的正确性建立在“单处理器 + 中断屏蔽”假设之上
//! - 对外暴露的接口已在内部完成必要的互斥与状态维护
//! - 调用者仍需遵守同步原语的使用约定（如成对 lock / unlock）
//!
//! ## Invariants
//! - 所有同步原语：
//!   - 内部状态仅能通过受控接口访问
//!   - 在阻塞当前任务前，内部状态必然已经更新
//! - 被加入等待队列的任务一定处于不可运行状态
//!
//! ## Behavior
//! - 各同步原语可被系统调用层或内核子系统直接使用
//! - 具体调度与唤醒行为由 `task` 模块负责
//! - 模块本身不感知具体的任务调度策略

mod condvar;
mod mutex;
mod semaphore;
mod up;

/// 条件变量
pub use condvar::Condvar;

/// 互斥锁抽象与实现
pub use mutex::{Mutex, MutexBlocking, MutexSpin};

/// 计数型信号量
pub use semaphore::Semaphore;

/// 单处理器内部可变性与中断屏蔽工具
pub use up::{UPIntrFreeCell, UPIntrRefMut, UPSafeCellRaw};
