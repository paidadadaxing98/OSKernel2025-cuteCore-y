//! # 互斥锁（Mutex）同步原语模块
//!
//! ## Overview
//! 本模块实现了内核中的互斥锁抽象及其两种具体实现：
//!
//! - `MutexSpin`：基于忙等 + 主动让出 CPU 的自旋互斥锁
//! - `MutexBlocking`：基于等待队列的阻塞型互斥锁
//!
//! 二者都实现了统一的 `Mutex` trait，以便在系统调用层和同步原语层
//! 以 **多态方式** 使用不同类型的互斥锁。
//!
//! ## Assumptions
//! - 系统运行在单处理器环境下
//! - 任务切换只能发生在显式调度点或中断返回时
//! - `UPIntrFreeCell` 能通过关中断提供足够的互斥性
//! - 所有任务都由调度器统一管理
//!
//! ## Safety
//! - 所有共享状态均被 `UPIntrFreeCell` 保护
//! - 在可能引发任务切换的路径上，均已释放内部可变借用
//! - `unsafe` 仅用于初始化阶段，且前提清晰
//!
//! ## Invariants
//! - 任意时刻：
//!   - 每把互斥锁最多只被一个任务持有
//!   - `MutexBlockingInner.locked == false` ⇒ 等待队列为空或即将被唤醒
//! - 等待队列中的任务一定处于阻塞状态
//!
//! ## Behavior
//! - `lock`：
//!   - 若互斥锁空闲，立即获得
//!   - 若被占用，根据实现选择“忙等”或“阻塞等待”
//! - `unlock`：
//!   - 若存在等待任务，唤醒其中一个
//!   - 否则释放互斥锁

use crate::sync::UPIntrFreeCell;
use crate::task::{
    block_current_and_run_next, current_task, suspend_current_and_run_next, wakeup_task,
    TaskControlBlock,
};
use alloc::collections::VecDeque;
use alloc::sync::Arc;

/// 互斥锁统一抽象接口
///
/// ## Overview
/// 所有互斥锁实现都必须满足该接口，以支持多态使用
///
/// ## Safety
/// - 实现者需保证：
///   - `lock` / `unlock` 的并发安全性
///   - 不会破坏调度器与任务状态一致性
pub trait Mutex: Sync + Send {
    /// 获取互斥锁，必要时阻塞或让出 CPU
    fn lock(&self);
    /// 释放互斥锁
    fn unlock(&self);
}

/// 自旋式互斥锁
///
/// ## Overview
/// - 不维护显式等待队列
/// - 当锁被占用时：
///   - 主动让出 CPU（`suspend_current_and_run_next`）
///   - 之后再次尝试获取锁
///
/// ## Assumptions
/// - 临界区较短
/// - 任务数量有限
///
/// ## Invariants
/// - `locked == true` 表示锁已被占用
pub struct MutexSpin {
    /// 锁状态，由 UPIntrFreeCell 保护
    locked: UPIntrFreeCell<bool>,
}

impl MutexSpin {
    /// 创建一个新的自旋互斥锁
    pub fn new() -> Self {
        Self {
            locked: unsafe { UPIntrFreeCell::new(false) },
        }
    }
}

impl Mutex for MutexSpin {
    /// 获取自旋锁
    ///
    /// ## Behavior
    /// - 若锁被占用：让出 CPU，稍后重试
    /// - 若锁空闲：直接获取
    fn lock(&self) {
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                // 已被占用，释放借用并让出 CPU
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                // 成功获取锁
                *locked = true;
                return;
            }
        }
    }

    /// 释放自旋锁
    fn unlock(&self) {
        let mut locked = self.locked.exclusive_access();
        *locked = false;
    }
}

/// 阻塞式互斥锁
///
/// ## Overview
/// - 当锁被占用时，将当前任务加入等待队列并阻塞
/// - 解锁时优先唤醒等待队列中的任务
///
/// ## Advantages
/// - 避免忙等
/// - 更适合临界区较长的场景
pub struct MutexBlocking {
    /// 内部状态，由 UPIntrFreeCell 保护
    inner: UPIntrFreeCell<MutexBlockingInner>,
}

/// 阻塞互斥锁的内部状态
///
/// ## Fields
/// - `locked`：当前是否被占用
/// - `wait_queue`：等待该锁的任务队列（FIFO）
pub struct MutexBlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    /// 创建一个新的阻塞互斥锁
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UPIntrFreeCell::new(MutexBlockingInner {
                    locked: false,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    /// 获取阻塞互斥锁
    ///
    /// ## Behavior
    /// - 若锁已被占用：
    ///   - 将当前任务加入等待队列
    ///   - 阻塞并触发任务切换
    /// - 若锁空闲：直接获取
    fn lock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            mutex_inner.locked = true;
        }
    }

    /// 释放阻塞互斥锁
    ///
    /// ## Behavior
    /// - 若等待队列非空：
    ///   - 唤醒队首任务（锁的所有权隐式转移）
    /// - 否则：
    ///   - 将锁标记为空闲
    ///
    /// ## Invariants
    /// - 调用 unlock 时，锁必须处于已上锁状态
    fn unlock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
}
