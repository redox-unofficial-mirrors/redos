#![no_std]

#[macro_use]
extern crate redos_abi;

#[no_mangle]
pub extern "C" fn main() {
    println!("Hello World!");
}
