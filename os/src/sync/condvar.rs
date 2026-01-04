//! # 条件变量（Condvar）同步原语模块
//!
//! ## Overview
//! 本模块实现了内核中的 **条件变量（Condition Variable）**，
//! 用于配合互斥锁实现“条件等待 / 条件通知”的同步模式。
//!
//! 条件变量本身 **不保存条件状态**，仅维护一个等待队列，
//! 具体条件判断需由使用者在互斥锁保护的临界区内完成。
//!
//! ## Assumptions
//! - 系统运行在单处理器环境下
//! - 所有并发仅来源于中断或显式调度点
//! - 条件变量总是与某个互斥锁配合使用
//!
//! ## Safety
//! - 条件变量内部状态通过 `UPIntrFreeCell` 保护
//! - 在阻塞当前任务前，已完成必要的状态入队
//! - `wait_with_mutex` 严格遵循“先释放锁，再睡眠，被唤醒后重新加锁”的语义
//!
//! ## Invariants
//! - `wait_queue` 中的任务一定处于阻塞状态
//! - 被 `signal` 唤醒的任务将从等待队列中移除
//! - 条件变量本身不保证唤醒顺序与条件成立
//!
//! ## Behavior
//! - `signal`：
//!   - 唤醒一个等待在该条件变量上的任务（若存在）
//! - `wait_*`：
//!   - 将当前任务加入等待队列并阻塞
//!   - 是否切换任务由具体接口决定

use crate::sync::{Mutex, UPIntrFreeCell};
use crate::task::{
    block_current_and_run_next, block_current_task, current_task, wakeup_task, TaskContext,
    TaskControlBlock,
};
use alloc::{collections::VecDeque, sync::Arc};

/// 条件变量
///
/// ## Overview
/// 对 `CondvarInner` 的安全封装，提供条件等待与唤醒接口
pub struct Condvar {
    /// 内部状态，由 UPIntrFreeCell 保护
    pub inner: UPIntrFreeCell<CondvarInner>,
}

/// 条件变量的内部状态
///
/// ## Fields
/// - `wait_queue`：
///   - 等待该条件变量的任务队列（FIFO）
pub struct CondvarInner {
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Condvar {
    /// 创建一个新的条件变量
    ///
    /// ## Invariants
    /// - 初始状态下等待队列为空
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UPIntrFreeCell::new(CondvarInner {
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    /// 唤醒一个等待在条件变量上的任务
    ///
    /// ## Behavior
    /// - 若等待队列非空：
    ///   - 唤醒队首任务
    /// - 若队列为空：
    ///   - 不执行任何操作
    pub fn signal(&self) {
        let mut inner = self.inner.exclusive_access();
        if let Some(task) = inner.wait_queue.pop_front() {
            wakeup_task(task);
        }
    }

    /// 在条件变量上等待，但 **不立即触发调度**
    ///
    /// ## Overview
    /// 该接口用于需要“延迟调度”或手动保存上下文的场景，
    /// 常见于底层调度或上下文切换逻辑中。
    ///
    /// ## Returns
    /// - 返回当前任务的 `TaskContext` 指针，用于后续恢复
    ///
    /// ## Safety
    /// - 调用者必须确保返回的上下文指针被正确使用
    pub fn wait_no_sched(&self) -> *mut TaskContext {
        self.inner.exclusive_session(|inner| {
            inner.wait_queue.push_back(current_task().unwrap());
        });
        block_current_task()
    }

    /// 在条件变量上等待，并配合互斥锁使用
    ///
    /// ## Behavior
    /// 该函数完整实现了经典的条件变量等待语义：
    ///
    /// ```text
    /// lock(mutex)
    /// while !condition {
    ///     condvar.wait(mutex)
    /// }
    /// unlock(mutex)
    /// ```
    ///
    /// 执行步骤如下：
    /// 1. 释放互斥锁
    /// 2. 将当前任务加入条件变量等待队列
    /// 3. 阻塞当前任务并触发调度
    /// 4. 被唤醒后重新获取互斥锁
    ///
    /// ## Safety
    /// - `mutex` 必须与该条件变量用于保护同一共享数据
    /// - 调用者需在外层自行检查条件是否满足（防止虚假唤醒）
    pub fn wait_with_mutex(&self, mutex: Arc<dyn Mutex>) {
        // 1. 释放互斥锁
        mutex.unlock();

        // 2. 加入条件变量等待队列
        self.inner.exclusive_session(|inner| {
            inner.wait_queue.push_back(current_task().unwrap());
        });

        // 3. 阻塞并调度
        block_current_and_run_next();

        // 4. 被唤醒后重新加锁
        mutex.lock();
    }
}
