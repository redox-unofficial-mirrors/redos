#![no_std]

use redos_abi::Abi;

#[no_mangle]
pub extern "C" fn main(abi: &Abi) {
    (abi.print)("Hello World!\n");
}

/// Required to handle panics
#[panic_handler]
#[no_mangle]
pub extern "C" fn rust_begin_unwind(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        //println!("REDOS PANIC:\n{}", info);
        loop {
            core::arch::asm!("hlt");
        }
    }
}
