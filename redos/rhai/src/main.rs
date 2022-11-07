#![no_std]

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate redos_abi;

use redos_abi::rt::abi;

mod rhai;

#[no_mangle]
pub extern "C" fn main() {
    rhai::rhai(&abi().args());
}
