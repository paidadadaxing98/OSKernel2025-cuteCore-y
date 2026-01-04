//! # 信号量（Semaphore）同步原语模块
//!
//! ## Overview
//! 本模块实现了内核中的 **计数型信号量（Counting Semaphore）**，
//! 用于管理对有限数量共享资源的并发访问。
//!
//! 信号量通过一个整数计数器与一个等待队列配合工作，
//! 支持典型的 `P / V`（或 `down / up`）操作语义。
//!
//! ## Assumptions
//! - 系统运行在单处理器环境下
//! - 任务并发仅来源于中断或显式调度点
//! - `UPIntrFreeCell` 能通过关中断保证临界区互斥
//!
//! ## Safety
//! - 所有对信号量内部状态的访问均被 `UPIntrFreeCell` 保护
//! - 在可能发生阻塞的路径上，已显式释放内部借用
//! - 等待队列中的任务均处于阻塞状态
//!
//! ## Invariants
//! - `count` 表示“可用资源数 − 等待任务数”
//! - `count < 0` 当且仅当存在等待任务
//! - 等待队列长度应与 `count` 的负值保持一致
//!
//! ## Behavior
//! - `down`：
//!   - 若资源不足，则阻塞当前任务
//! - `up`：
//!   - 释放资源，并在必要时唤醒等待任务

use crate::sync::UPIntrFreeCell;
use crate::task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock};
use alloc::{collections::VecDeque, sync::Arc};

/// 信号量类型
///
/// ## Overview
/// 对 `SemaphoreInner` 的安全封装，对外提供 `up / down` 接口
pub struct Semaphore {
    /// 内部状态，由 UPIntrFreeCell 保护
    pub inner: UPIntrFreeCell<SemaphoreInner>,
}

/// 信号量的内部状态
///
/// ## Fields
/// - `count`：
///   - 当前可用资源计数
///   - 允许为负数，用于表示等待任务数量
/// - `wait_queue`：
///   - 等待该信号量的任务队列（FIFO）
pub struct SemaphoreInner {
    pub count: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    /// 创建一个新的信号量
    ///
    /// ## Parameters
    /// - `res_count`：初始可用资源数量
    ///
    /// ## Invariants
    /// - 初始状态下：
    ///   - `count == res_count`
    ///   - 等待队列为空
    pub fn new(res_count: usize) -> Self {
        Self {
            inner: unsafe {
                UPIntrFreeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    /// 执行 V 操作（up）
    ///
    /// ## Behavior
    /// - 增加资源计数
    /// - 若存在等待任务（`count <= 0`）：
    ///   - 唤醒队首任务
    pub fn up(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                wakeup_task(task);
            }
        }
    }

    /// 执行 P 操作（down）
    ///
    /// ## Behavior
    /// - 尝试获取资源（`count -= 1`）
    ///   - 若资源不足（`count < 0`）：
    ///     - 将当前任务加入等待队列
    ///     - 阻塞当前任务并触发调度
    pub fn down(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
            block_current_and_run_next();
        }
    }
}
