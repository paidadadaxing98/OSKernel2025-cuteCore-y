#[cfg(feature = "riscv")]
pub mod riscv;

#[cfg(feature = "riscv")]
pub use riscv::{
    bootstrap_init,
    config::{
        KERNEL_HEAP_SIZE, KERNEL_STACK_SIZE, MEMORY_END, PAGE_SIZE, PAGE_SIZE_BITS, TRAMPOLINE,
        TRAP_CONTEXT_BASE, USER_STACK_SIZE, BLOCK_SZ,
    },
    kernel_stack::{kstack_alloc, KernelStack, ustack_bottom_from_tid, trap_cx_bottom_from_tid},
    machine_init,
    sbi::{console_flush, console_getchar, console_putchar, shutdown},
    sync::INTR_MASKING_INFO,
    timer::{get_clock_freq, get_time},
    trap::{trap_handler, trap_return, context::TrapContext},
    switch::__switch,
    PageTableEntryImpl, PageTableImpl,
};

#[cfg(feature = "loongarch")]
pub mod loongarch;

#[cfg(feature = "loongarch")]
pub use loongarch::{
    bootstrap_init,
    config::{
        HIGH_BASE_EIGHT, KERNEL_HEAP_SIZE, KERNEL_STACK_SIZE, MEMORY_END, MEMORY_HIGH_BASE,
        MEMORY_HIGH_BASE_VPN, MEMORY_SIZE, PAGE_SIZE, PAGE_SIZE_BITS, PALEN, TRAMPOLINE,
        TRAP_CONTEXT_BASE, USER_STACK_SIZE, VA_MASK, VPN_SEG_MASK,
    },
    kernel_stack::{kstack_alloc, KernelStack},
    machine_init,
    sbi::{console_flush, console_getchar, console_putchar, shutdown},
    sync::INTR_MASKING_INFO,
    timer::{get_clock_freq, get_time},
    trap::{trap_handler, trap_return,
           context::TrapContext
    },
    PageTableEntryImpl, PageTableImpl,
};
