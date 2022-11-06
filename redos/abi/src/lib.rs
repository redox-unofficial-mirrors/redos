#![no_std]

#[repr(C)]
pub struct Abi {
    pub print: extern "C" fn(&str),
}
