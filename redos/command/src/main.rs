#![no_std]

#[macro_use]
extern crate redos_abi;

use redos_abi::rt::abi;

#[no_mangle]
pub extern "C" fn main() {
    for (i, arg) in abi().args().iter().enumerate() {
        println!("{}: {}", i, arg);
    }
}
