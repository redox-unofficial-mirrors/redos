#![no_std]

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate redos_abi;

mod vi;

#[no_mangle]
pub extern "C" fn main() {
    //TODO: args to main
    let args = &["autoexec"];
    match vi::vi(args) {
        Ok(()) => (),
        Err(err) => {
            println!("vi: error: {}", err);
        }
    }
}
