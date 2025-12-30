use crate::hal::arch::riscv::timer::set_next_trigger;

pub mod trap;
pub mod sbi;
pub mod boot;
pub mod timer;
pub mod config;
pub mod sv39;

pub fn bootstrap_init() {}

pub fn machine_init() {
    trap::init();
    trap::enable_timer_interrupt();
    set_next_trigger();
    println!("RISC-V machine init completed.");
}

pub type PageTableImpl = sv39::SV39PageTable;
pub type PageTableEntryImpl = sv39::PageTableEntry;