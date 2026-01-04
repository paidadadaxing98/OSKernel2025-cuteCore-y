//! # 同步与时间相关系统调用模块
//!
//! ## Overview
//! 本模块实现了一组内核态系统调用接口，主要面向用户进程提供：
//! - 线程/任务的休眠（sleep）
//! - 互斥锁（Mutex）的创建、加锁与解锁
//! - 信号量（Semaphore）的创建、P/V 操作
//! - 条件变量（Condvar）的创建、等待与唤醒
//!
//! 这些系统调用以 **进程私有资源表** 的形式管理同步原语，
//! 每个进程维护独立的 mutex / semaphore / condvar 列表。
//!
//! ## Assumptions
//! - 当前系统支持多任务调度，并且任务可以被阻塞与唤醒
//! - `current_process()` 与 `current_task()` 始终在系统调用上下文中有效
//! - 进程内部的同步对象列表（如 `mutex_list`）大小受限且索引合法
//! - 所有同步原语都通过 `Arc` 在内核中安全共享
//!
//! ## Safety
//! - 所有对进程内部资源表的访问都通过 `inner_exclusive_access()` 完成，
//!   保证互斥访问，防止并发修改
//! - 在调用可能阻塞的操作（如 `lock` / `down` / `wait`）前，
//!   显式释放进程内部锁，避免死锁
//! - 未对用户传入的 `id` 做越界检查，假定用户态保证合法
//!
//! ## Invariants
//! - 进程的各类同步对象列表中：
//!   - `Some(Arc<T>)` 表示已分配对象
//!   - `None` 表示可复用的空槽位
//! - 系统调用返回的 `id` 永远对应进程私有列表中的索引
//!
//! ## Behavior
//! - 所有系统调用成功时返回 `0` 或合法资源 ID
//! - 阻塞类系统调用会触发任务切换
//! - 不负责对象的显式销毁（依赖进程退出时统一回收）

#![allow(unused)]

use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;

/// 使当前任务休眠指定的毫秒数
///
/// ## Behavior
/// - 计算任务的唤醒时间点
/// - 将当前任务加入定时器队列
/// - 阻塞当前任务并切换到下一个可运行任务
///
/// ## Safety
/// - 当前任务必须存在
/// - 定时器系统必须正确维护唤醒逻辑
pub fn sys_sleep(ms: usize) -> isize {
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}

/// 创建一个互斥锁
///
/// ## Parameters
/// - `blocking`：
///   - `false` 表示自旋锁（忙等）
///   - `true` 表示阻塞锁（睡眠等待）
///
/// ## Returns
/// - 返回互斥锁在当前进程 mutex 表中的 ID
///
/// ## Invariants
/// - 同一进程中，不同 mutex ID 对应不同互斥锁实例
pub fn sys_mutex_create(blocking: bool) -> isize {
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}

/// 对指定互斥锁加锁
///
/// ## Behavior
/// - 若互斥锁已被占用，当前任务可能被阻塞
///
/// ## Safety
/// - 在调用 `lock()` 前释放进程内部锁，避免死锁
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.lock();
    0
}

/// 对指定互斥锁解锁
///
/// ## Invariants
/// - 调用者应当是该互斥锁的持有者
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}

/// 创建一个信号量
///
/// ## Parameters
/// - `res_count`：信号量初始资源数量
///
/// ## Returns
/// - 信号量在进程 semaphore 表中的 ID
pub fn sys_semaphore_create(res_count: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}

/// 对信号量执行 V 操作（up）
///
/// ## Behavior
/// - 增加资源计数
/// - 若存在等待任务，可能唤醒其中一个
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();
    0
}

/// 对信号量执行 P 操作（down）
///
/// ## Behavior
/// - 若资源不足，当前任务将被阻塞
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.down();
    0
}

/// 创建一个条件变量
///
/// ## Returns
/// - 条件变量在进程 condvar 表中的 ID
pub fn sys_condvar_create() -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}

/// 唤醒一个等待在条件变量上的任务
///
/// ## Behavior
/// - 若无任务等待，则该操作为空操作
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}

/// 在条件变量上等待，并释放指定互斥锁
///
/// ## Behavior
/// - 原子地释放互斥锁并阻塞当前任务
/// - 被唤醒后重新获取互斥锁
///
/// ## Safety
/// - 必须保证 mutex 与 condvar 属于同一进程
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait_with_mutex(mutex);
    0
}
