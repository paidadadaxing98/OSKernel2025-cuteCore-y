//! 物理页帧分配器模块。
//!
//! 本模块负责管理内核可用的物理页帧（Physical Frames），
//! 提供页帧的分配与回收接口，是内存管理子系统的基础组件之一。
//!
//! # Overview
//! - 使用页帧号（`PhysPageNum`）作为最小分配单位
//! - 提供全局页帧分配器 `FRAME_ALLOCATOR`
//! - 通过 RAII 语义自动回收页帧
//!
//! # Allocation Strategy
//! - 当前实现为基于栈（Stack）的页帧分配器
//! - 支持顺序分配与回收页帧
//! - 使用 recycled 列表复用已释放页帧
//!
//! # Safety
//! - 本模块包含全局可变状态
//! - 所有访问必须通过 `UPIntrFreeCell` 串行化
//! - 调用方必须保证在正确的初始化顺序下使用
//!
//! # Invariants
//! - 已分配的页帧不会被重复分配
//! - 被回收的页帧只能回收一次
//! - `FrameTracker` 生命周期与页帧占用严格绑定



use super::{PhysAddr, PhysPageNum};
use crate::hal::MEMORY_END;
use crate::sync::UPIntrFreeCell;
use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};
use lazy_static::*;



lazy_static! {
    /// 全局物理页帧分配器。
    ///
    /// 使用 `UPIntrFreeCell` 包裹以保证在单核 / 关中断环境下的
    /// 独占访问。
    ///
    /// INVARIANT:
    /// - 所有页帧分配与回收必须通过该分配器完成
    /// - 在任意时刻，分配器内部状态是自洽的
    pub static ref FRAME_ALLOCATOR: UPIntrFreeCell<FrameAllocatorImpl> =
        unsafe { UPIntrFreeCell::new(FrameAllocatorImpl::new()) };
}


/// 初始化物理页帧分配器。
///
/// 页帧管理范围：
/// - 起始地址：内核镜像结束地址（`ekernel`）
/// - 结束地址：系统物理内存上限（`MEMORY_END`）
///
/// SAFETY:
/// - `ekernel` 由链接脚本提供，地址有效
/// - 初始化函数只会在系统启动阶段调用一次
/// - 初始化期间不会发生并发页帧访问
pub fn init_frame_allocator() {
    extern "C" {
        fn ekernel();
    }
    FRAME_ALLOCATOR.exclusive_access().init(
        PhysAddr::from(ekernel as *const () as usize).ceil(),
        PhysAddr::from(MEMORY_END).floor(),
    );
}


/// 分配一个物理页帧。
///
/// 成功时返回一个 `FrameTracker`，
/// 其生命周期与页帧占用绑定。
pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(FrameTracker::new)
}


/// 一次性分配多个连续页帧。
///
/// 返回的每个页帧都由对应的 `FrameTracker` 管理。
pub fn frame_alloc_more(num: usize) -> Option<Vec<FrameTracker>> {
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc_more(num)
        .map(|x| x.iter().map(|&t| FrameTracker::new(t)).collect())
}


/// 回收一个物理页帧。
///
/// 通常由 `FrameTracker::drop` 自动调用，
/// 不建议手动使用。
pub fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}


/// 页帧跟踪器（RAII 封装）。
///
/// `FrameTracker` 表示对一个物理页帧的所有权：
/// - 创建时表示页帧被分配
/// - 被 drop 时自动回收页帧
#[derive(Clone)]
pub struct FrameTracker {
    /// 被管理的物理页帧号
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    /// 创建一个新的页帧跟踪器。
    ///
    /// 在创建时会对整个页帧进行清零，
    /// 以避免泄露旧数据。
    pub fn new(ppn: PhysPageNum) -> Self {
        // 清空页帧内容
        let bytes_array = ppn.get_bytes_array();
        for i in bytes_array {
            *i = 0;
        }
        Self { ppn }
    }
}

/// 实现页帧跟踪器的调试输出。
impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}

/// 当 `FrameTracker` 被销毁时，
/// 自动将页帧归还给分配器。
impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}


/// 页帧分配器抽象接口。
///
/// 不同分配策略（如 bitmap / buddy / stack）
/// 都可以实现该 trait。
trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn alloc_more(&mut self, pages: usize) -> Option<Vec<PhysPageNum>>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}

/// 基于栈的页帧分配器实现。
///
/// 分配策略：
/// - 顺序分配未使用页帧
/// - 回收的页帧放入 recycled 栈中复用
pub struct StackFrameAllocator {
    /// 当前尚未分配的起始页帧号
    current: usize,
    /// 可分配页帧的上界（不包含）
    end: usize,
    /// 已回收、可再次分配的页帧号
    recycled: Vec<usize>,
}

impl StackFrameAllocator {
    /// 初始化页帧分配区间。
    ///
    /// `[l, r)` 区间内的页帧将被纳入管理。
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
    }
}
impl FrameAllocator for StackFrameAllocator {
    /// 创建一个新的栈式页帧分配器。
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }

    /// 分配一个页帧。
    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            Some(ppn.into())
        } else if self.current == self.end {
            None
        } else {
            self.current += 1;
            Some((self.current - 1).into())
        }
    }

    /// 分配多个连续页帧。
    fn alloc_more(&mut self, pages: usize) -> Option<Vec<PhysPageNum>> {
        if self.current + pages >= self.end {
            None
        } else {
            self.current += pages;
            let arr: Vec<usize> = (1..pages + 1).collect();
            let v = arr.iter().map(|x| (self.current - x).into()).collect();
            Some(v)
        }
    }

    /// 回收一个页帧。
    ///
    /// 会进行合法性检查，防止重复回收或非法回收。
    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        // 合法性检查
        if ppn >= self.current || self.recycled.iter().any(|&v| v == ppn) {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
        // 回收页帧
        self.recycled.push(ppn);
    }
}

/// 当前使用的页帧分配器实现。
type FrameAllocatorImpl = StackFrameAllocator;
