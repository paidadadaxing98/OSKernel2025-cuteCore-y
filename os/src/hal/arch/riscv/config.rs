#![allow(unused)]

pub const PAGE_SIZE: usize = 0x1000; // 4KB
pub const PAGE_SIZE_BITS: usize = 0xc; // 4KB = 2^12 Bytes

pub const USER_STACK_SIZE: usize = PAGE_SIZE * 0x40; // 256KB
pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 0x40; // 256KB
// INFO: 暂时定位 16 MB 大小的内核堆
pub const KERNEL_HEAP_SIZE: usize = PAGE_SIZE * 0x4000; // 16MB

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT_BASE: usize = TRAMPOLINE - PAGE_SIZE;

pub const MEMORY_START: usize = 0x0000_0000_8000_0000;
pub const MEMORY_SIZE: usize = 0x3000_0000;
#[cfg(feature = "board_rvqemu")]
pub const MEMORY_END: usize = MEMORY_START + MEMORY_SIZE; // 0x0000_0000_b000_0000
