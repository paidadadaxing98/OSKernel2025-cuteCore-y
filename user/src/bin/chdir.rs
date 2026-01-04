#![no_std]
#![no_main]

extern crate alloc;
extern crate user;

use user::{println, chdir};

#[no_mangle]
fn main() -> i32 {
    let path = "/bin\0";
    match chdir(path) {
        0 => println!("Changed directory to {}", path),
        _ => println!("Failed to change directory to {}", path),
    }
    0
}