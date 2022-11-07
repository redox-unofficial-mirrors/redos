#![no_std]

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate redos_abi;

#[no_mangle]
pub extern "C" fn main() {
    let string = format!("this is a test of formatting");
    println!("{}", string);
}
