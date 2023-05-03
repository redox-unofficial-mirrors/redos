#![no_std]
#![feature(associated_type_defaults)]
#![feature(lang_items)]
#![cfg_attr(
    target_os = "uefi",
    no_main,
    feature(control_flow_enum),
    feature(try_trait_v2)
)]

#[cfg_attr(target_os = "none", macro_use)]
extern crate alloc;

#[cfg(target_os = "uefi")]
#[macro_use]
extern crate uefi_std as std;

use alloc::string::String;
use core::{
    fmt::{self, Write},
    ptr, slice, str,
};
use redoxfs::Disk;

use self::os::{Os, OsDisk, OsKey, OsMemoryKind, OsVideoMode, VideoModeIter};

#[macro_use]
mod os;

#[cfg(feature = "serial_debug")]
mod serial_16550;

mod dos;

const KIBI: usize = 1024;
const MIBI: usize = KIBI * KIBI;

pub(crate) static mut LIVE_OPT: Option<(u64, &'static [u8])> = None;

struct SliceWriter<'a> {
    slice: &'a mut [u8],
    i: usize,
}

impl<'a> Write for SliceWriter<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            if let Some(slice_b) = self.slice.get_mut(self.i) {
                *slice_b = b;
                self.i += 1;
            } else {
                return Err(fmt::Error);
            }
        }
        Ok(())
    }
}

fn redoxfs<D: Disk, V: Iterator<Item = OsVideoMode>>(
    os: &mut dyn Os<D, V>,
) -> (redoxfs::FileSystem<D>, Option<&'static [u8]>) {
    let attempts = 10;
    for attempt in 0..=attempts {
        let mut password_opt = None;
        if attempt > 0 {
            print!("\rRedoxFS password ({}/{}): ", attempt, attempts);

            let mut password = String::new();

            loop {
                match os.get_key() {
                    OsKey::Backspace | OsKey::Delete => {
                        if !password.is_empty() {
                            print!("\x08 \x08");
                            password.pop();
                        }
                    }
                    OsKey::Char(c) => {
                        print!("*");
                        password.push(c)
                    }
                    OsKey::Enter => break,
                    _ => (),
                }
            }

            // Erase password information
            while os.get_text_position().0 > 0 {
                print!("\x08 \x08");
            }

            if !password.is_empty() {
                password_opt = Some(password);
            }
        }
        match os.filesystem(password_opt.as_ref().map(|x| x.as_bytes())) {
            Ok(fs) => {
                return (
                    fs,
                    password_opt.map(|password| {
                        // Copy password to page aligned memory
                        let password_size = password.len();
                        let password_base = os.alloc_zeroed_page_aligned(password_size);
                        unsafe {
                            ptr::copy(password.as_ptr(), password_base, password_size);
                            slice::from_raw_parts(password_base, password_size)
                        }
                    }),
                );
            }
            Err(err) => match err.errno {
                // Incorrect password, try again
                syscall::ENOKEY => (),
                _ => {
                    panic!("Failed to open RedoxFS: {}", err);
                }
            },
        }
    }
    panic!("RedoxFS out of unlock attempts");
}

fn elf_entry(data: &[u8]) -> (u64, bool) {
    match (data[4], data[5]) {
        // 32-bit, little endian
        (1, 1) => (
            u32::from_le_bytes(
                <[u8; 4]>::try_from(&data[0x18..0x18 + 4]).expect("conversion cannot fail"),
            ) as u64,
            false,
        ),
        // 32-bit, big endian
        (1, 2) => (
            u32::from_be_bytes(
                <[u8; 4]>::try_from(&data[0x18..0x18 + 4]).expect("conversion cannot fail"),
            ) as u64,
            false,
        ),
        // 64-bit, little endian
        (2, 1) => (
            u64::from_le_bytes(
                <[u8; 8]>::try_from(&data[0x18..0x18 + 8]).expect("conversion cannot fail"),
            ),
            true,
        ),
        // 64-bit, big endian
        (2, 2) => (
            u64::from_be_bytes(
                <[u8; 8]>::try_from(&data[0x18..0x18 + 8]).expect("conversion cannot fail"),
            ),
            true,
        ),
        (ei_class, ei_data) => {
            panic!("Unsupported ELF EI_CLASS {} EI_DATA {}", ei_class, ei_data);
        }
    }
}

fn main<O: Os<OsDisk, VideoModeIter> + 'static>(mut os: O) -> ! {
    println!(
        "Redox OS Bootloader {} on {}",
        env!("CARGO_PKG_VERSION"),
        os.name()
    );

    let (fs, _password_opt) = redoxfs(&mut os);

    print!("RedoxFS ");
    for i in 0..fs.header.uuid().len() {
        if i == 4 || i == 6 || i == 8 || i == 10 {
            print!("-");
        }

        print!("{:>02x}", fs.header.uuid()[i]);
    }
    println!(": {} MiB", fs.header.size() / MIBI as u64);
    println!();

    dos::main(os, fs);
}
