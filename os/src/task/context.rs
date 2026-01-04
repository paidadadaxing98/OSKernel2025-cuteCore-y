//! # 任务上下文（TaskContext）模块
//!
//! ## Overview
//! 本模块定义了 **任务上下文（Task Context）** 的数据结构及其初始化方式，
//! 用于在内核态进行任务切换时保存和恢复 CPU 的关键寄存器状态。
//!
//! `TaskContext` 是调度器进行上下文切换（context switch）的核心数据结构，
//! 通常保存在 `TaskControlBlock` 中，并在任务被切换出 / 切换入 CPU 时使用。
//!
//! ## Assumptions
//! - 当前体系结构遵循约定的寄存器调用规范
//! - 任务切换发生在内核态
//! - `trap_return` 是一个合法的内核入口函数，用于返回用户态
//!
//! ## Safety
//! - `TaskContext` 使用 `#[repr(C)]` 保证内存布局稳定，
//!   以便与汇编实现的上下文切换代码正确配合
//! - 所有字段均为纯数据，不包含指针生命周期问题
//!
//! ## Invariants
//! - `ra` 始终表示任务恢复执行时的返回地址
//! - `sp` 始终指向该任务对应的内核栈顶
//! - `s` 数组保存的是需要跨函数调用保持的通用寄存器
//!
//! ## Behavior
//! - `zero_init`：
//!   - 构造一个“空上下文”，通常用于占位或初始化
//! - `goto_trap_return`：
//!   - 构造一个在首次调度时直接返回用户态的任务上下文

use crate::hal::trap_return;

/// 任务上下文
///
/// ## Overview
/// 保存任务在内核态切换时所需的最小寄存器集合，
/// 用于支持抢占式或协作式任务调度。
///
/// ## Fields
/// - `ra`：
///   - 返回地址寄存器（Return Address）
///   - 在 RISC-V 中对应 `ra`
/// - `sp`：
///   - 栈指针（Stack Pointer）
///   - 指向任务的内核栈
/// - `s`：
///   - 被调用者保存寄存器（Saved Registers）
///   - 在 RISC-V 中对应 `s0 ~ s11`
#[repr(C)]
pub struct TaskContext {
    // 返回地址，在la中应该为$ra
    ra: usize,
    // 栈指针，在la中应该为$sp
    sp: usize,
    // 通用寄存器，在la中应该为$s0~$s8
    s: [usize; 12],
}

impl TaskContext {
    /// 构造一个全零初始化的任务上下文
    ///
    /// ## Overview
    /// 用于创建一个尚未绑定具体执行流的上下文，
    /// 常见于任务结构体的初始占位。
    ///
    /// ## Invariants
    /// - 所有寄存器字段均为 0
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    /// 构造一个“首次运行即返回用户态”的任务上下文
    ///
    /// ## Parameters
    /// - `kstack_ptr`：
    ///   - 任务对应的内核栈栈顶指针
    ///
    /// ## Behavior
    /// - 设置返回地址为 `trap_return`
    /// - 当该任务第一次被调度执行时：
    ///   - 会直接跳转到 `trap_return`
    ///   - 由内核完成从内核态到用户态的切换
    ///
    /// ## Safety
    /// - `kstack_ptr` 必须指向合法且已分配的内核栈空间
    pub fn goto_trap_return(kstack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
