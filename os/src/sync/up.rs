//! # 单处理器安全内部可变性封装模块
//!
//! ## Overview
//! 本模块提供了若干用于 **单处理器（Uniprocessor, UP）环境** 下的
//! 内部可变性（interior mutability）封装工具，用于在内核中安全地
//! 访问全局或静态数据结构。
//!
//! 模块主要包含三类封装：
//! - `UPSafeCellRaw`：基于 `UnsafeCell` 的最底层封装，完全由使用者保证安全
//! - `UPIntrFreeCell`：在访问期间自动关闭中断，防止中断打断导致的数据竞争
//! - `UPIntrRefMut`：配合 `UPIntrFreeCell` 使用的 RAII 可变借用守卫
//!
//! ## Assumptions
//! - 系统运行在单核处理器环境中
//! - 不存在真正的并行执行，仅可能被中断打断
//! - 中断屏蔽可以提供足够的互斥保证
//! - `INTR_MASKING_INFO` 能正确维护中断嵌套状态
//!
//! ## Safety
//! - `unsafe impl Sync` 的正确性完全依赖“单处理器 + 中断屏蔽”这一前提
//! - `UPSafeCellRaw` 不做任何借用或并发检查，误用将直接导致未定义行为
//! - `UPIntrFreeCell` 通过中断屏蔽 + `RefCell` 动态借用检查，提供更强安全性
//!
//! ## Invariants
//! - 在任意时刻：
//!   - 若某个 `UPIntrFreeCell` 处于可变借用状态，则中断必然被屏蔽
//!   - 当 `UPIntrRefMut` 被 drop 时，中断状态一定会被恢复
//!
//! ## Behavior
//! - 所有 `exclusive_access` 调用都会返回独占可变访问
//! - 使用 RAII 保证中断屏蔽与恢复成对出现
//! - 借用冲突将直接 panic（`RefCell` 语义）

use crate::hal::INTR_MASKING_INFO;
use core::cell::{RefCell, RefMut, UnsafeCell};
use core::ops::{Deref, DerefMut};

/// 基于 `UnsafeCell` 的最底层 UP 内部可变性封装
///
/// ## Overview
/// 提供对内部数据的可变访问，但 **不进行任何安全检查**
///
/// ## Safety
/// - 使用者必须保证：
///   - 仅在单处理器环境下使用
///   - 不会出现并发或中断竞争
///
/// ## Invariants
/// - 内部数据只能通过 `get_mut` 访问
pub struct UPSafeCellRaw<T> {
    /// 内部实际存储的数据
    inner: UnsafeCell<T>,
}

/// 声明其在 UP 场景下是线程安全的（由使用者保证）
unsafe impl<T> Sync for UPSafeCellRaw<T> {}

impl<T> UPSafeCellRaw<T> {
    /// 创建一个新的 `UPSafeCellRaw`
    ///
    /// ## Safety
    /// - 调用者必须保证后续访问满足 UP 假设
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }

    /// 获取内部数据的可变引用
    ///
    /// ## Safety
    /// - 不进行任何借用或并发检查
    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut (*self.inner.get()) }
    }
}

/// 在访问期间自动关闭中断的 UP 内部可变性封装
///
/// ## Overview
/// 使用 `RefCell` 提供动态借用检查，
/// 并在进入临界区时屏蔽中断，防止中断导致的数据竞争
///
/// ## Safety
/// - 适用于单处理器 + 中断并发模型
pub struct UPIntrFreeCell<T> {
    /// 内部数据，通过 RefCell 实现内部可变性
    inner: RefCell<T>,
}

/// 声明其在 UP + 中断屏蔽前提下是安全的
unsafe impl<T> Sync for UPIntrFreeCell<T> {}

/// 新增：声明其可以跨线程/核心安全转移
/// 因为访问时会关闭中断，保证了独占性
unsafe impl<T> Send for UPIntrFreeCell<T> {}

/// `UPIntrFreeCell` 的可变借用守卫
///
/// ## Overview
/// - 通过 RAII 管理中断屏蔽生命周期
/// - Drop 时自动恢复中断
///
/// ## Invariants
/// - 生命周期内：中断始终被屏蔽
pub struct UPIntrRefMut<'a, T>(Option<RefMut<'a, T>>);

impl<T> UPIntrFreeCell<T> {
    /// 创建一个新的 `UPIntrFreeCell`
    ///
    /// ## Safety
    /// - 使用者需保证仅在 UP 环境下使用
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }

    /// 获取内部数据的独占访问权
    ///
    /// ## Behavior
    /// - 屏蔽中断
    /// - 获取 RefCell 的可变借用
    /// - 若发生借用冲突将 panic
    pub fn exclusive_access(&self) -> UPIntrRefMut<'_, T> {
        INTR_MASKING_INFO.get_mut().enter();
        UPIntrRefMut(Some(self.inner.borrow_mut()))
    }

    /// 在独占访问会话中执行闭包
    ///
    /// ## Behavior
    /// - 自动管理中断屏蔽与恢复
    /// - 提供更安全、简洁的访问方式
    pub fn exclusive_session<F, V>(&self, f: F) -> V
    where
        F: FnOnce(&mut T) -> V,
    {
        let mut inner = self.exclusive_access();
        f(inner.deref_mut())
    }
}

/// 在 `UPIntrRefMut` 生命周期结束时恢复中断
impl<'a, T> Drop for UPIntrRefMut<'a, T> {
    fn drop(&mut self) {
        self.0 = None;
        INTR_MASKING_INFO.get_mut().exit();
    }
}

impl<'a, T> Deref for UPIntrRefMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap().deref()
    }
}
impl<'a, T> DerefMut for UPIntrRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().unwrap().deref_mut()
    }
}
