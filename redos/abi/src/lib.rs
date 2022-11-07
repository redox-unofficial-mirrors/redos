#![no_std]
#![cfg_attr(feature = "alloc", feature(alloc_error_handler))]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::{
    string::String,
    vec::Vec,
};

pub use syscall::error;

use crate::error::{Error, Result};

#[cfg(feature = "alloc")]
pub mod allocator;

#[cfg(feature = "rt")]
pub mod rt;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct AbiData(pub usize);

#[derive(Clone, Default)]
#[repr(C)]
pub struct AbiTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub nanosecond: u32,
}

#[repr(C)]
pub struct Abi {
    pub abi_data: AbiData,
    pub abi_args: *const (*const u8, usize),
    pub abi_print: extern "C" fn(data: AbiData, ptr: *const u8, len: usize),
    pub abi_text_clear: extern "C" fn(data: AbiData),
    pub abi_text_color: extern "C" fn(data: AbiData, color: u8),
    pub abi_key: extern "C" fn(data: AbiData) -> char,
    pub abi_time: extern "C" fn(data: AbiData, time: *mut AbiTime),
    pub abi_alloc: extern "C" fn(data: AbiData, size: usize, align: usize) -> *mut u8,
    pub abi_dealloc: extern "C" fn(data: AbiData, ptr: *mut u8, size: usize, align: usize),
    pub abi_open: extern "C" fn(data: AbiData, ptr: *const u8, len: usize) -> usize,
    pub abi_pread: extern "C" fn(data: AbiData, fd: usize, ptr: *mut u8, len: usize, offset: usize) -> usize,
    pub abi_pwrite: extern "C" fn(data: AbiData, fd: usize, ptr: *const u8, len: usize, offset: usize) -> usize,
    pub abi_truncate: extern "C" fn(data: AbiData, fd: usize, size: usize) -> usize,
}

impl Abi {
    #[cfg(feature = "alloc")]
    pub fn args(&self) -> Vec<&'static str> {
        use core::{slice, str};

        //TODO: need this many unsafes?
        let mut args = Vec::new();
        for arg_i in 0..4096 { // TODO: is this a good limit?
            let (ptr, len) = unsafe { *self.abi_args.add(arg_i) };
            if ptr.is_null() {
                break;
            }

            let slice = unsafe { slice::from_raw_parts(ptr, len) };
            let arg = str::from_utf8(slice).unwrap();
            args.push(arg);
        }
        args
    }

    pub fn print(&self, s: &str) {
        (self.abi_print)(self.abi_data, s.as_ptr(), s.len());
    }

    pub fn text_clear(&self) {
        (self.abi_text_clear)(self.abi_data);
    }

    pub fn text_color(&self, color: u8) {
        (self.abi_text_color)(self.abi_data, color);
    }

    pub fn key(&self) -> char {
        (self.abi_key)(self.abi_data)
    }

    #[cfg(feature = "alloc")]
    pub fn read_line(&self) -> String {
        let mut line = String::new();
        loop {
            // Print cursor
            self.text_color(0x70);
            print!(" \x08");
            self.text_color(0x07);

            let key = self.key();

            // Wipe out cursor
            print!(" \x08");

            match key {
                '\x08' | '\x7F' => if ! line.is_empty() {
                    print!("\x08 \x08");
                    line.pop();
                },
                '\n' | '\r'=> {
                    break;
                },
                _ => {
                    print!("{}", key);
                    line.push(key)
                },
            }
        }
        line
    }

    pub fn time(&self, time: &mut AbiTime) {
        (self.abi_time)(self.abi_data, time as *mut AbiTime)
    }

    pub fn open(&self, path: &str) -> Result<usize> {
        Error::demux((self.abi_open)(self.abi_data, path.as_ptr(), path.len()))
    }

    pub fn pread(&self, fd: usize, buf: &mut [u8], offset: usize) -> Result<usize> {
        Error::demux((self.abi_pread)(self.abi_data, fd, buf.as_mut_ptr(), buf.len(), offset))
    }

    #[cfg(feature = "alloc")]
    pub fn read_to_end(&self, fd: usize) -> Result<Vec<u8>> {
        let mut offset = 0;

        //TODO: evaluate block size
        let block_size = 4096;

        let mut data = Vec::new();
        loop {
            // Resize so data can hold one more block
            data.resize(offset + block_size, 0);

            // Read next block
            let count = self.pread(fd, &mut data[offset..], offset)?;

            if count == 0 {
                // End of file
                data.truncate(offset);
                return Ok(data)
            }

            // Adjust offset for next block
            offset += count;
        }
    }

    #[cfg(feature = "alloc")]
    pub fn read_to_string(&self, fd: usize) -> Result<String> {
        let data = self.read_to_end(fd)?;
        String::from_utf8(data).map_err(|_| Error::new(error::EINVAL))
    }

    pub fn pwrite(&self, fd: usize, buf: &[u8], offset: usize) -> Result<usize> {
        Error::demux((self.abi_pwrite)(self.abi_data, fd, buf.as_ptr(), buf.len(), offset))
    }

    pub fn truncate(&self, fd: usize, size: usize) -> Result<usize> {
        Error::demux((self.abi_truncate)(self.abi_data, fd, size))
    }
}
