#![no_std]

#[cfg(feature = "rt")]
pub mod rt;

#[repr(C)]
pub struct Abi {
    pub print: extern "C" fn(&str),
}
