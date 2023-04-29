#![no_std]

#[macro_use]
extern crate redos_abi;

use hwio::{Io, Pio, WriteOnly};
use redos_abi::{AbiTime, rt::abi};
use redos_hw::sb16::Sb16;

pub const MILLIS_PER_SEC: u128 = 1_000;
pub const MICROS_PER_SEC: u128 = 1_000_000;
pub const NANOS_PER_SEC: u128 = 1_000_000_000;

fn time_to_nanos(time: &AbiTime) -> u128 {
    // Unix time from clock
    let mut secs: u64 = (time.year as u64 - 1970) * 31_536_000;

    let mut leap_days = (time.year as u64 - 1972) / 4 + 1;
    if time.year % 4 == 0 && time.month <= 2 {
        leap_days -= 1;
    }
    secs += leap_days * 86_400;

    match time.month {
        2 => secs += 2_678_400,
        3 => secs += 5_097_600,
        4 => secs += 7_776_000,
        5 => secs += 10_368_000,
        6 => secs += 13_046_400,
        7 => secs += 15_638_400,
        8 => secs += 18_316_800,
        9 => secs += 20_995_200,
        10 => secs += 23_587_200,
        11 => secs += 26_265_600,
        12 => secs += 28_857_600,
        _ => (),
    }

    secs += (time.day as u64 - 1) * 86_400;
    secs += time.hour as u64 * 3600;
    secs += time.minute as u64 * 60;
    secs += time.second as u64;

    (secs as u128) * NANOS_PER_SEC + (time.nanosecond as u128)
}

fn delay_us(micros: u32) {
    let mut time = AbiTime::default();
    abi().time(&mut time);
    let start = time_to_nanos(&time);
    loop {
        abi().time(&mut time);
        if (time_to_nanos(&time) - start) >= ((micros as u128) * (NANOS_PER_SEC / MICROS_PER_SEC)) {
            break;
        }
    }
}

#[no_mangle]
pub extern "C" fn main() {
    let mut blaster = match unsafe { Sb16::new(0x220, delay_us) } {
        Ok(ok) => ok,
        Err(err) => {
            println!("Failed to find Sound Blaster 16: {}", err);
            return;
        }
    };

    println!("Sound Blaster 16 DSP {}.{:02}", blaster.version.0, blaster.version.1);
}
