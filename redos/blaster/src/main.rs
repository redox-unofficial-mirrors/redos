#![no_std]

#[macro_use]
extern crate redos_abi;

use core::slice;
use hwio::{Io, Pio, WriteOnly};
use redos_abi::rt::abi;
use redos_hw::sb16::Sb16;

#[no_mangle]
pub extern "C" fn main() {
    let sample_fd = match abi().open("redos/at-dooms-gate.raw") {
        Ok(ok) => ok,
        Err(err) => {
            println!("Failed to open sample file: {}", err);
            return;
        }
    };

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

    let mut top_half = true;
    let mut sample_offset = 0;
    loop {
        let half_len = blaster.buffer.len() / 2;
        let buffer = if top_half {
            &mut blaster.buffer[half_len..]
        } else {
            &mut blaster.buffer[..half_len]
        };
        let count = match abi().pread(sample_fd, buffer, sample_offset) {
            Ok(ok) => ok,
            Err(err) => {
                println!("Failed to read sample file: {}", err);
                break;
            }
        };
        if count == 0 {
            break;
        }

        if sample_offset == 0 {
            match blaster.start() {
                Ok(()) => (),
                Err(err) => {
                    println!("Failed to start playback from Sound Blaster 16: {}", err);
                    break;
                }
            }
        }

        sample_offset += count;
        top_half = !top_half;

        blaster.wait();
    }

    match blaster.stop() {
        Ok(()) => (),
        Err(err) => {
            println!("Failed to stop playback from Sound Blaster 16: {}", err);
        }
    }
}
