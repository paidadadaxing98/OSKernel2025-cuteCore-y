use crate::hal::shutdown;
use crate::task::current_kstack_top;
use core::arch::asm;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("\n[kernel] PANIC!");
    if let Some(location) = info.location() {
        println!(
            "[kernel] panicked at {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
    }
    if let Some(msg) = info.message() {
        println!("[kernel] Message: {}", msg);
    }
    backtrace();
    shutdown()
}

fn backtrace() {
    let mut fp: usize;
    let stop = current_kstack_top();
    unsafe {
        asm!("mv {}, s0", out(reg) fp);
    }
    println!("\n----START BACKTRACE----");
    for i in 0..10 {
        if fp == stop {
            break;
        }
        unsafe {
            println!("#{}:ra={:#x}", i, *((fp - 8) as *const usize));
            fp = *((fp - 16) as *const usize);
        }
    }
    println!("----END OF BACKTRACE----");
}

#[macro_export]
macro_rules! color_text {
    ($text:expr, $color:expr) => {{
        format_args!("\x1b[{}m{}\x1b[0m", $color, $text)
    }};
}

pub trait Bytes<T> {
    fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<T>();
        unsafe {
            core::slice::from_raw_parts(self as *const _ as *const T as usize as *const u8, size)
        }
    }

    fn as_bytes_mut(&mut self) -> &mut [u8] {
        let size = core::mem::size_of::<T>();
        unsafe {
            core::slice::from_raw_parts_mut(self as *mut _ as *mut T as usize as *mut u8, size)
        }
    }
}
