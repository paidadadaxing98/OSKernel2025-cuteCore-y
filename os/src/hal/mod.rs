pub mod arch;
mod paltform;

pub use arch::{bootstrap_init, machine_init};
pub use arch::{console_putchar, console_getchar, console_flush, shutdown};
pub use arch::{get_time, get_clock_freq};
pub use arch::{USER_STACK_SIZE, KERNEL_HEAP_SIZE, KERNEL_STACK_SIZE, PAGE_SIZE, PAGE_SIZE_BITS, TRAMPOLINE, TRAP_CONTEXT_BASE, MEMORY_END};
pub use arch::{PageTableImpl, PageTableEntryImpl, PTEFlags};

pub use paltform::{MMIO, CLOCK_FREQ};