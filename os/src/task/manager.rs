//! # 任务管理与进程映射模块
//!
//! ## Overview
//! 本模块实现了内核中的 **任务管理器（TaskManager）** 以及
//! **进程 ID 到进程控制块（PID → PCB）映射表**，
//! 是调度器和进程管理子系统的重要基础组成部分。
//!
//! 主要职责包括：
//! - 维护就绪任务队列（ready queue）
//! - 提供任务的加入、唤醒与获取接口
//! - 维护 PID 到 `ProcessControlBlock` 的全局映射
//!
//! 所有全局状态均通过 `UPIntrFreeCell` 进行保护，
//! 以适配 **单处理器 + 中断并发模型**。
//!
//! ## Assumptions
//! - 系统运行在单处理器环境
//! - 任意时刻只有一个 CPU 执行内核代码
//! - 并发仅来源于中断或显式调度点
//!
//! ## Safety
//! - 所有全局可变数据均由 `UPIntrFreeCell` 保护
//! - 在修改任务状态后，才将任务加入就绪队列
//! - PID 映射表的插入与删除遵循严格的生命周期约定
//!
//! ## Invariants
//! - 就绪队列中的任务：
//!   - 其 `task_status` 一定为 `Ready`
//! - 同一个 PID 在 `PID2PCB` 中最多对应一个进程
//! - 被移除的 PID 必然曾经存在于映射表中
//!
//! ## Behavior
//! - 任务调度采用 FIFO 顺序（简单就绪队列）
//! - 模块本身不实现时间片或优先级策略
//! - 调度策略可在此基础上扩展

use crate::sync::UPIntrFreeCell;
use crate::task::process::ProcessControlBlock;
use crate::task::task::TaskStatus;
use crate::task::{current_task, TaskControlBlock};
use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use lazy_static::lazy_static;

lazy_static! {
    /// 全局任务管理器
    ///
    /// ## Overview
    /// 维护系统中所有处于就绪状态的任务队列
    pub static ref TASK_MANAGER: UPIntrFreeCell<TaskManager> =
        unsafe { UPIntrFreeCell::new(TaskManager::new()) };

    /// PID → ProcessControlBlock 映射表
    ///
    /// ## Overview
    /// 用于通过进程 ID 快速定位对应的进程控制块
    pub static ref PID2PCB: UPIntrFreeCell<BTreeMap<usize, Arc<ProcessControlBlock>>> =
        unsafe { UPIntrFreeCell::new(BTreeMap::new()) };
}

/// 将一个任务加入就绪队列
///
/// ## Behavior
/// - 不检查任务状态，由调用者保证其合法性
pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}

/// 唤醒一个任务并加入就绪队列
///
/// ## Behavior
/// - 将任务状态设置为 `Ready`
/// - 将任务加入就绪队列
///
/// ## Invariants
/// - 被唤醒的任务此前应处于阻塞状态
pub fn wakeup_task(task: Arc<TaskControlBlock>) {
    let mut task_inner = task.inner_exclusive_access();
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    add_task(task);
}

/// 从就绪队列中取出一个任务
///
/// ## Returns
/// - `Some(task)`：
///   - 当前可运行的任务
/// - `None`：
///   - 当前无可运行任务
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}

/// 根据 PID 获取对应的进程控制块
///
/// ## Returns
/// - `Some(Arc<ProcessControlBlock>)`：
///   - PID 对应的进程存在
/// - `None`：
///   - 未找到对应进程
pub fn pid2process(pid: usize) -> Option<Arc<ProcessControlBlock>> {
    let map = PID2PCB.exclusive_access();
    map.get(&pid).map(Arc::clone)
}

/// 向 PID 映射表中插入一个进程
///
/// ## Invariants
/// - 同一个 PID 不应被重复插入
pub fn insert_into_pid2process(pid: usize, process: Arc<ProcessControlBlock>) {
    PID2PCB.exclusive_access().insert(pid, process);
}

/// 从 PID 映射表中移除一个进程
///
/// ## Panics
/// - 若 PID 不存在，则直接 panic，
///   表示内核内部状态不一致
pub fn remove_from_pid2process(pid: usize) {
    let mut map = PID2PCB.exclusive_access();
    if map.remove(&pid).is_none() {
        panic!("cannot find pid {} in pid2task!", pid);
    }
}

/// 任务管理器
///
/// ## Overview
/// 维护一个简单的 FIFO 就绪队列，
/// 为调度器提供最基础的任务管理能力
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl TaskManager {
    /// 创建一个新的任务管理器
    ///
    /// ## Invariants
    /// - 初始就绪队列为空
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }

    /// 将任务加入就绪队列
    ///
    /// ## Behavior
    /// - 采用 FIFO 顺序
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }

    /// 从就绪队列中取出一个任务
    ///
    /// ## Behavior
    /// - 返回队首任务
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
    pub fn find_by_pid(&self, pid: usize) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.iter().find_map(|task| {
            // 获取任务的进程引用
            let process = task.process.upgrade()?;
            // 检查进程的 PID 是否匹配
            if process.pid.0 == pid {
                Some(Arc::clone(task))
            } else {
                None
            }
        })
    }
}
pub fn find_task_by_pid(pid: usize) -> Option<Arc<TaskControlBlock>> {
    // 获取当前任务
    let task = current_task().unwrap();
    // 如果当前任务的pid与指定的pid相同，返回当前任务
    if task.process.upgrade().unwrap().pid.0 == pid {
        Some(task)
    } else {
        // 否则从任务管理器中查找
        TASK_MANAGER.exclusive_access().find_by_pid(pid)
    }
}
pub fn wake_blocked(task: Arc<TaskControlBlock>) {
    let mut task_inner = task.inner_exclusive_access();
    if task_inner.task_status == TaskStatus::Blocked {
        task_inner.task_status = TaskStatus::Ready;
        drop(task_inner);
        add_task(task);
    }
}
