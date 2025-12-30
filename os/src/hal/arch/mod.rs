#[cfg(feature = "riscv")]
pub mod riscv;

#[cfg(feature = "riscv")]
pub use riscv::{
    bootstrap_init, machine_init,
    sbi::{console_getchar, console_putchar, console_flush, shutdown},
    timer::{get_time, get_clock_freq},
    config::{USER_STACK_SIZE, KERNEL_HEAP_SIZE, KERNEL_STACK_SIZE, PAGE_SIZE, PAGE_SIZE_BITS, TRAMPOLINE, TRAP_CONTEXT_BASE, MEMORY_END},
    sv39::PTEFlags,
    PageTableImpl, PageTableEntryImpl,
};



#[cfg(feature = "loongarch")]
pub mod loongarch;