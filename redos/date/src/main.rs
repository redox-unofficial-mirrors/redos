#![no_std]

#[macro_use]
extern crate redos_abi;

use redos_abi::{rt::abi, AbiTime};

#[no_mangle]
pub extern "C" fn main() {
    let mut time = AbiTime::default();
    abi().time(&mut time);
    println!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        time.year, time.month, time.day, time.hour, time.minute, time.second
    );
}
