use core::fmt::{Write, Result};

pub struct Printer;

impl Write for Printer {
    fn write_str(&mut self, s: &str) -> Result {
        unsafe {
            #[cfg(feature = "serial_debug")]
            {
                crate::os::serial::COM1.write_str(s)?;
            }
            crate::os::VGA.write_str(s)?;
        }

        Ok(())
    }
}

/// Print to console
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = write!($crate::os::macros::Printer, $($arg)*);
    });
}

/// Print with new line to console
#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}
