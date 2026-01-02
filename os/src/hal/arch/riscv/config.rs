#![allow(unused)]

pub const PAGE_SIZE: usize = 0x1000; // 4KB
pub const PAGE_SIZE_BITS: usize = 0xc; // 4KB = 2^12 Bytes

pub const USER_STACK_SIZE: usize = PAGE_SIZE * 2; // 8KB
pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 2; // 8KB

pub const KERNEL_HEAP_SIZE: usize = PAGE_SIZE * 0x4000; // 16MB

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT_BASE: usize = TRAMPOLINE - PAGE_SIZE;

pub const MEMORY_END: usize = 0x8800_0000;
pub const BLOCK_SZ: usize = 512;
