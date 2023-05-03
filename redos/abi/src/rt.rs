use core::{fmt, ptr};
use log::{LevelFilter, Log, Metadata, Record};

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

static LOGGER: Logger = Logger;

struct Logger;

impl Logger {
    fn init(&'static self) {
        log::set_logger(self).unwrap();
        log::set_max_level(LevelFilter::Debug);
    }
}

impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

#[no_mangle]
pub unsafe extern "C" fn start(abi: *const Abi) {
    extern "C" {
        fn main();
    }
    ABI = abi;
    LOGGER.init();
    main();
}

/// Required to handle panics
#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        println!("REDOS PANIC:\n{}", info);
        loop {
            core::arch::asm!("hlt");
        }
    }
}
