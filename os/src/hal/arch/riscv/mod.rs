use crate::hal::arch::riscv::timer::set_next_trigger;

pub mod boot;
pub mod config;
pub mod kernel_stack;
pub mod sbi;
pub mod sv39;
pub mod switch;
pub mod sync;
pub mod timer;
pub mod trap;

pub fn bootstrap_init() {}

pub fn machine_init() {
    trap::init();
    trap::enable_timer_interrupt();
    set_next_trigger();
}

pub type PageTableImpl = sv39::SV39PageTable;
pub type PageTableEntryImpl = sv39::PageTableEntry;
