#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]


extern crate core;
extern crate alloc;


#[macro_use]
pub mod console;
mod hal;
mod lang_items;
mod timer;
mod task;
mod mm;

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    hal::bootstrap_init();
    console::init();
    println!("Welcome to RustOS!");
    mm::init();
    println!("Memory management initialized.");
    hal::machine_init();
    panic!("Shouldn't get here!");
}