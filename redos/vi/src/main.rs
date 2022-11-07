#![no_std]

#[macro_use]
extern crate redos_abi;

mod vi;

#[no_mangle]
pub extern "C" fn main() {
    match vi::vi() {
        Ok(()) => (),
        Err(err) => {
            println!("vi: error: {}", err);
        }
    }
}
