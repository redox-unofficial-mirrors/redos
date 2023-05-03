#![no_std]

#[macro_use]
extern crate redos_abi;

use core::slice;
use hwio::{Io, Pio, WriteOnly};
use redos_abi::rt::abi;
use redos_hw::sb16::Sb16;

#[no_mangle]
pub extern "C" fn main() {
    let mut blaster = match unsafe { Sb16::new(0x220, |micros| abi().delay_us(micros)) } {
        Ok(ok) => ok,
        Err(err) => {
            println!("Failed to find Sound Blaster 16: {}", err);
            return;
        }
    };

    println!(
        "Sound Blaster 16 DSP {}.{:02}",
        blaster.version.0, blaster.version.1
    );

    println!("ISA DMA buffer: {:p}", blaster.buffer.as_ptr());
    let mut buffer = unsafe {
        slice::from_raw_parts_mut(
            blaster.buffer.as_mut_ptr() as *mut [i16; 2],
            blaster.buffer.len() / 4,
        )
    };

    for (i, sample) in buffer.iter_mut().enumerate() {
        if i % 50 == 0 {
            sample[0] = 32767;
            sample[1] = 32767;
        } else {
            sample[0] = -32767;
            sample[1] = -32767;
        }
    }
}
