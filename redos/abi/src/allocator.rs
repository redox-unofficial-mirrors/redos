use core::alloc::{GlobalAlloc, Layout};

use crate::rt::abi;

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

pub struct Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let abi = abi();
        (abi.abi_alloc)(abi.abi_data, layout.size(), layout.align())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let abi = abi();
        (abi.abi_dealloc)(abi.abi_data, ptr, layout.size(), layout.align());
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("failed to allocate {:?}", layout)
}
