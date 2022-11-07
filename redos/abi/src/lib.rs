#![no_std]

#[cfg(feature = "alloc")]
pub mod alloc;

#[cfg(feature = "rt")]
pub mod rt;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct AbiData(pub usize);

#[repr(C)]
pub struct Abi {
    pub abi_data: AbiData,
    pub abi_print: extern "C" fn(AbiData, *const u8, usize),
}

impl Abi {
    pub fn print(&self, s: &str) {
        (self.abi_print)(self.abi_data, s.as_ptr(), s.len());
    }
}
