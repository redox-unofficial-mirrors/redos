//TODO: move to redos_abi?

use core::{fmt, ptr};

use crate::Abi;

static mut ABI: *const Abi = ptr::null_mut();

pub fn abi() -> &'static Abi {
    unsafe { &*ABI }
}

pub struct AbiPrint;

impl fmt::Write for AbiPrint {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        abi().print(s);
        Ok(())
    }
}

/// Print to console
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = write!($crate::rt::AbiPrint, $($arg)*);
    });
}

/// Print with new line to console
#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

#[no_mangle]
pub unsafe extern "C" fn start(abi: *const Abi) {
    extern "C" {
        fn main();
    }
    ABI = abi;
    main();
}

/// Required to handle panics
#[panic_handler]
#[no_mangle]
pub extern "C" fn rust_begin_unwind(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        println!("REDOS PANIC:\n{}", info);
        loop {
            core::arch::asm!("hlt");
        }
    }
}
