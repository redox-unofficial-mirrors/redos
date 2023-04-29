#![no_std]

#[macro_use]
extern crate redos_abi;

use hwio::{Io, Pio, WriteOnly};
use redos_abi::{AbiTime, rt::abi};
use redos_hw::{pcspkr::PcSpeaker, pit::Pit};

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

fn delay(millis: u32) {
    let mut time = AbiTime::default();
    abi().time(&mut time);
    let start = time_to_nanos(&time);
    loop {
        abi().time(&mut time);
        if (time_to_nanos(&time) - start) >= ((millis as u128) * (NANOS_PER_SEC / 1000)) {
            break;
        }
    }
}

fn print_time(time: &AbiTime) {
    println!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}", time.year, time.month, time.day, time.hour, time.minute, time.second, time.nanosecond / 1_000_000);
}

const C2: u32 = 65;
const Db2: u32 = 69;
const D2: u32 = 73;
const E2: u32 = 82;
const F2: u32 = 87;
const G2: u32 = 98;
const A2: u32 = 110;
const Bb2: u32 = 117;
const B2: u32 = 123;

const C3: u32 = 130;
const Db3: u32 = 139;
const D3: u32 = 147;
const Eb3: u32 = 156;
const E3: u32 = 165;
const F3: u32 = 175;
const G3: u32 = 196;
const Ab3: u32 = 208;
const A3: u32 = 220;
const B3: u32 = 247;

const C4: u32 = 262;
const Db4: u32 = 277;
const D4: u32 = 294;
const E4: u32 = 330;
const F4: u32 = 349;
const G4: u32 = 392;
const A4: u32 = 440;
const B4: u32 = 494;

const C5: u32 = 523;
const D5: u32 = 587;
const E5: u32 = 659;
const F5: u32 = 698;
const G5: u32 = 784;
const A5: u32 = 880;
const B5: u32 = 988;

#[no_mangle]
pub extern "C" fn main() {
    let mut pc_speaker = PcSpeaker::new();
    let mut pit = Pit::new();

    pc_speaker.start();

    let mut note = move |freq, millis| {
        pit.chan_2_freq(freq);
        pc_speaker.start();
        delay(millis);
        pc_speaker.stop();
    };

    let mut time = AbiTime::default();
    abi().time(&mut time);
    print_time(&time);

    const SIXTEENTH: u32 = EIGHTH / 2;
    const EIGHTH: u32 = QUARTER / 2;
    const QUARTER: u32 = 60000 / 30;
    const HALF: u32 = QUARTER * 2;

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(E3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(D3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(C3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(Bb2, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(B2, EIGHTH);
    note(C3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(E3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(D3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(C3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(Bb2, HALF);


    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(E3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(D3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(C3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(Bb2, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(B2, EIGHTH);
    note(C3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(E3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(D3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);

    note(B3, SIXTEENTH);
    note(G3, SIXTEENTH);
    note(E3, SIXTEENTH);
    note(G3, SIXTEENTH);
    note(B3, SIXTEENTH);
    note(G3, SIXTEENTH);
    note(B3, SIXTEENTH);
    note(E4, SIXTEENTH);
    note(B3, SIXTEENTH);
    note(G3, SIXTEENTH);
    note(B3, SIXTEENTH);
    note(G3, SIXTEENTH);
    note(B3, SIXTEENTH);
    note(E4, SIXTEENTH);
    note(G4, SIXTEENTH);
    note(B4, SIXTEENTH);


    note(A2, EIGHTH);
    note(A2, EIGHTH);
    note(A3, EIGHTH);

    note(A2, EIGHTH);
    note(A2, EIGHTH);
    note(G3, EIGHTH);

    note(A2, EIGHTH);
    note(A2, EIGHTH);
    note(F3, EIGHTH);

    note(A2, EIGHTH);
    note(A2, EIGHTH);
    note(Eb3, EIGHTH);

    note(A2, EIGHTH);
    note(A2, EIGHTH);
    note(E3, EIGHTH);
    note(F3, EIGHTH);

    note(A2, EIGHTH);
    note(A2, EIGHTH);
    note(A3, EIGHTH);

    note(A2, EIGHTH);
    note(A2, EIGHTH);
    note(G3, EIGHTH);

    note(A2, EIGHTH);
    note(A2, EIGHTH);
    note(F3, EIGHTH);

    note(A2, EIGHTH);
    note(A2, EIGHTH);
    note(Eb3, HALF);


    note(A2, EIGHTH);
    note(A2, EIGHTH);
    note(A3, EIGHTH);

    note(A2, EIGHTH);
    note(A2, EIGHTH);
    note(G3, EIGHTH);

    note(A2, EIGHTH);
    note(A2, EIGHTH);
    note(F3, EIGHTH);

    note(A2, EIGHTH);
    note(A2, EIGHTH);
    note(Eb3, EIGHTH);

    note(A2, EIGHTH);
    note(A2, EIGHTH);
    note(E3, EIGHTH);
    note(F3, EIGHTH);

    note(A2, EIGHTH);
    note(A2, EIGHTH);
    note(A3, EIGHTH);

    note(A2, EIGHTH);
    note(A2, EIGHTH);
    note(G3, EIGHTH);

    note(A2, EIGHTH);
    note(A2, EIGHTH);

    note(A4, SIXTEENTH);
    note(E4, SIXTEENTH);
    note(E4, SIXTEENTH);
    note(A4, SIXTEENTH);
    note(E4, SIXTEENTH);
    note(C4, SIXTEENTH);
    note(E4, SIXTEENTH);
    note(A4, SIXTEENTH);
    note(C5, SIXTEENTH);
    note(A4, SIXTEENTH);
    note(E4, SIXTEENTH);
    note(A4, SIXTEENTH);
    note(E4, SIXTEENTH);
    note(A4, SIXTEENTH);
    note(E4, SIXTEENTH);
    note(C4, SIXTEENTH);


    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(E3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(D3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(C3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(Bb2, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(B2, EIGHTH);
    note(C3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(E3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(D3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(C3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(Bb2, HALF);


    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(E3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(D3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(C3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(Bb2, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(B2, EIGHTH);
    note(C3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(E3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(D3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(C3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(Bb2, HALF);


    note(Db3, EIGHTH);
    note(Db3, EIGHTH);
    note(Db4, EIGHTH);

    note(Db3, EIGHTH);
    note(Db3, EIGHTH);
    note(B3, EIGHTH);

    note(Db3, EIGHTH);
    note(Db3, EIGHTH);
    note(A3, EIGHTH);

    note(Db3, EIGHTH);
    note(Db3, EIGHTH);
    note(G3, EIGHTH);

    note(Db3, EIGHTH);
    note(Db3, EIGHTH);
    note(Ab3, EIGHTH);
    note(A3, EIGHTH);

    note(B2, EIGHTH);
    note(B2, EIGHTH);
    note(B3, EIGHTH);

    note(B2, EIGHTH);
    note(B2, EIGHTH);
    note(A3, EIGHTH);

    note(B2, EIGHTH);
    note(B2, EIGHTH);
    note(G3, EIGHTH);

    note(A2, EIGHTH);
    note(A2, EIGHTH);
    note(F3, HALF);


    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(E3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(D3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(C3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(Bb2, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(B2, EIGHTH);
    note(C3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(E3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);
    note(D3, EIGHTH);

    note(E2, EIGHTH);
    note(E2, EIGHTH);

    note(G4, SIXTEENTH);
    note(B4, SIXTEENTH);
    note(E4, SIXTEENTH);
    note(B3, SIXTEENTH);
    note(G4, SIXTEENTH);
    note(E4, SIXTEENTH);
    note(B4, SIXTEENTH);
    note(G4, SIXTEENTH);
    note(B4, SIXTEENTH);
    note(G4, SIXTEENTH);
    note(E4, SIXTEENTH);
    note(B3, SIXTEENTH);
    note(G4, SIXTEENTH);
    note(B4, SIXTEENTH);
    note(E5, SIXTEENTH);
    note(G5, SIXTEENTH);

    abi().time(&mut time);
    print_time(&time);
}
