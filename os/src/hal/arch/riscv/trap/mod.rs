mod context;

use core::arch::global_asm;
use riscv::register::{scause, sie, sscratch, sstatus, stval, stvec};
use riscv::register::mtvec::TrapMode;
use riscv::register::scause::{Interrupt, Trap};
use crate::hal::TRAMPOLINE;

pub use context::TrapContext;
use crate::hal::arch::riscv::timer::set_next_trigger;
use crate::timer::check_timer;

global_asm!(include_str!("trap.S"));

pub fn init() {
    set_kernel_trap_entry();
}

fn set_kernel_trap_entry() {
    extern "C" {
        fn __alltraps();
        fn __alltraps_k();
    }
    let __alltraps_k_va = __alltraps_k as *const() as usize - __alltraps as *const() as usize + TRAMPOLINE;
    unsafe {
        stvec::write(__alltraps_k_va, TrapMode::Direct);
        sscratch::write(trap_from_kernel as usize);
    }
}

#[no_mangle]
pub fn trap_from_kernel(_trap_cx: &TrapContext) {
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorExternal) => {
            // crate::board::irq_handler();
            todo!()
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            check_timer();
            // do not schedule now
        }
        _ => {
            panic!(
                "Unsupported trap from kernel: {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
}


pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

fn enable_supervisor_interrupt() {
    unsafe {
        sstatus::set_sie();
    }
}

fn disable_supervisor_interrupt() {
    unsafe {
        sstatus::clear_sie();
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE, TrapMode::Direct);
    }
}


#[no_mangle]
pub fn trap_handler() -> ! {
    trap_return();
    unreachable!()
}

pub fn trap_return() -> ! {
    todo!()
}













