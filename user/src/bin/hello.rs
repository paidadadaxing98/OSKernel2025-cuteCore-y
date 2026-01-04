#![no_std]
#![no_main]

extern crate user;

use user::println;



#[no_mangle]
fn main() -> i32 {
    println!("Hello, RustOS!");
    0
}