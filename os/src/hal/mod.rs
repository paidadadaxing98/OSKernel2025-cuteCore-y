pub mod arch;
mod platform;

pub use arch::kstack_alloc;
pub use arch::INTR_MASKING_INFO;
pub use arch::{bootstrap_init, machine_init};
pub use arch::{console_flush, console_getchar, console_putchar, shutdown};
pub use arch::{get_clock_freq, get_time};
pub use arch::{trap_handler, trap_return};
pub use arch::{KernelStack, PageTableEntryImpl, PageTableImpl, TrapContext};
pub use arch::{
    KERNEL_HEAP_SIZE, KERNEL_STACK_SIZE, MEMORY_END, PAGE_SIZE, PAGE_SIZE_BITS, TRAMPOLINE,
    TRAP_CONTEXT_BASE, USER_STACK_SIZE, BLOCK_SZ,
};
pub use arch::{ustack_bottom_from_tid, trap_cx_bottom_from_tid};
pub use arch::__switch;

#[cfg(feature = "loongarch")]
pub use arch::{
    HIGH_BASE_EIGHT, MEMORY_HIGH_BASE, MEMORY_HIGH_BASE_VPN, MEMORY_SIZE, PALEN, VA_MASK,
    VPN_SEG_MASK,
};

#[cfg(feature = "board_laqemu")]
pub use platform::{MEM_SIZE, MMIO};

#[cfg(feature = "board_rvqemu")]
pub use platform::{CLOCK_FREQ, MMIO};

#[cfg(feature = "board_2k1000")]
pub use platform::{MEM_SIZE, MMIO};
