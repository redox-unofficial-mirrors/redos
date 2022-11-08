use syscall::{Io, Pio};

use crate::serial_16550::SerialPort;

pub static mut COM1: SerialPort<Pio<u8>> = SerialPort::<Pio<u8>>::new(0x3F8);
pub static mut COM2: SerialPort<Pio<u8>> = SerialPort::<Pio<u8>>::new(0x2F8);
pub static mut COM3: SerialPort<Pio<u8>> = SerialPort::<Pio<u8>>::new(0x3E8);
pub static mut COM4: SerialPort<Pio<u8>> = SerialPort::<Pio<u8>>::new(0x2E8);
