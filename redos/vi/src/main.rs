#![no_std]

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate redos_abi;

use redos_abi::rt::abi;

mod vi;

#[no_mangle]
pub extern "C" fn main() {
    match vi::vi(&abi().args()) {
        Ok(()) => (),
        Err(err) => {
            println!("vi: error: {}", err);
        }
    }
}
