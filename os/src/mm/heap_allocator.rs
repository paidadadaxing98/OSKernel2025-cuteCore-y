//! 内核堆内存分配模块。
//!
//! 本模块负责：
//! - 定义并初始化内核全局堆分配器
//! - 提供堆内存分配失败时的错误处理逻辑
//!
//! 内核堆基于 `buddy_system_allocator` 实现，
//! 所有动态内存分配（如 `Box`、`Vec`、`Arc` 等）
//! 最终都会通过本模块进行。
//!
//! # Overview
//! - 使用一段静态内存作为内核堆空间
//! - 通过 `LockedHeap` 管理堆内存
//! - 在系统启动早期完成初始化
//!
//! # Safety
//! - 本模块包含 `unsafe` 代码，用于操作裸内存
//! - 调用方必须保证：
//!   - `init_heap` 只被调用一次
//!   - 初始化完成后才允许进行堆分配
//!
//! # Invariants
//! - 内核堆空间在初始化后不可移动
//! - 堆分配器在整个系统生命周期内保持有效


use crate::hal::KERNEL_HEAP_SIZE;
use buddy_system_allocator::LockedHeap;
use core::ptr::addr_of_mut;


/// 内核全局堆分配器。
///
/// 使用 `buddy_system_allocator` 提供的 `LockedHeap`，
/// 作为全局内存分配器供整个内核使用。
///
/// INVARIANT:
/// - 在系统生命周期内只会被初始化一次
/// - 所有堆分配操作必须通过该分配器完成
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();


/// 堆内存分配失败处理函数。
///
/// 当内核发生堆分配失败（如内存耗尽或对齐要求无法满足）时，
/// 该函数会被调用。
///
/// 该实现会输出详细的分配请求信息，
/// 然后直接触发 panic，终止内核执行。
#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    println!("Heap allocation error:");
    println!("  Requested size: {} bytes", layout.size());
    println!("  Requested alignment: {} bytes", layout.align());
    println!("  Total heap size: {} bytes", KERNEL_HEAP_SIZE);
    println!(
        "  Requested size in MB: {:.2} MB",
        layout.size() as f64 / (1024.0 * 1024.0)
    );
    println!(
        "  Total heap size in MB: {:.2} MB",
        KERNEL_HEAP_SIZE as f64 / (1024.0 * 1024.0)
    );

    panic!("Heap allocation error, layout = {:?}", layout);
}

/// 内核堆的实际内存空间。
///
/// 使用一段静态分配的数组作为内核堆的后端存储。
///
/// SAFETY:
/// - 该静态内存仅在 `init_heap` 中被初始化
/// - 初始化完成后，其管理权完全交由堆分配器
static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];


/// 初始化内核堆。
///
/// 该函数必须在系统启动早期调用，
/// 并且只能调用一次。
///
/// SAFETY:
/// - `HEAP_SPACE` 是一段有效且连续的内存
/// - 在调用该函数之前，不得进行任何堆分配
/// - 初始化期间不会发生并发访问
pub fn init_heap() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(addr_of_mut!(HEAP_SPACE) as usize, KERNEL_HEAP_SIZE);
    }
}
