#![no_std]
#![no_main]

extern crate alloc;
extern crate user;

use user::println;
use user::getcwd;

#[no_mangle]
fn main() -> i32 {
    let mut buffer = [0u8; 128];
    let ret = getcwd(&mut buffer) as usize;

    if ret != 0 {
        println!("Successfully got current working directory");
    }

    0
}