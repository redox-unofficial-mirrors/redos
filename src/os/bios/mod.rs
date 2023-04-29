use alloc::alloc::{alloc_zeroed, GlobalAlloc, Layout};
use core::{
    convert::TryFrom,
    ptr::NonNull,
    slice,
};
use linked_list_allocator::Heap;
use redos_abi::AbiTime;

use crate::os::{Os, OsKey, OsVideoMode};

use self::memory_map::memory_map;
use self::thunk::ThunkData;
use self::vga::Vga;

pub use self::disk::DiskBios as OsDisk;
pub use self::memory_map::MemoryMapIter;
pub use self::vbe::VideoModeIter;

#[macro_use]
pub(crate) mod macros;

mod disk;
mod memory_map;
mod panic;
#[cfg(feature = "serial_debug")]
pub(crate) mod serial;
mod thunk;
mod time;
mod vbe;
mod vga;

// Real mode memory allocation, for use with thunk
// 0x500 to 0x7BFF is free
const DISK_BIOS_ADDR: usize = 0x70000; // 64 KiB at 448 KiB, ends at 512 KiB
const VBE_CARD_INFO_ADDR: usize = 0x1000; // 512 bytes, ends at 0x11FF
const VBE_MODE_INFO_ADDR: usize = 0x1200; // 256 bytes, ends at 0x12FF
const VBE_EDID_ADDR: usize = 0x1300; // 128 bytes, ends at 0x137F
const MEMORY_MAP_ADDR: usize = 0x1380; // 24 bytes, ends at 0x1397
const DISK_ADDRESS_PACKET_ADDR: usize = 0x1398; // 16 bytes, ends at 0x13A7
const THUNK_STACK_ADDR: usize = 0x7C00; // Grows downwards
const VGA_ADDR: usize = 0xB8000;

pub(crate) static mut HEAP: Heap = Heap::empty();

pub struct Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HEAP
            .allocate_first_fit(layout)
            .ok()
            .map_or(core::ptr::null_mut(), |allocation| allocation.as_ptr())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        HEAP
            .deallocate(NonNull::new_unchecked(ptr), layout)
    }
}

#[global_allocator]
pub static ALLOCATOR: Allocator = Allocator;

pub(crate) static mut VGA: Vga = unsafe { Vga::new(VGA_ADDR, 80, 25) };

pub struct OsBios {
    boot_disk: usize,
    thunk10: extern "C" fn(),
    thunk13: extern "C" fn(),
    thunk15: extern "C" fn(),
    thunk16: extern "C" fn(),
    thunk1a: extern "C" fn(),
}

impl Os<
    OsDisk,
    VideoModeIter
> for OsBios {
    fn name(&self) -> &str {
        "x86/BIOS"
    }

    fn alloc_zeroed_page_aligned(&self, size: usize) -> *mut u8 {
        assert!(size != 0);

        let page_size = self.page_size();
        let pages = (size + page_size - 1) / page_size;

        let ptr = unsafe {
            alloc_zeroed(Layout::from_size_align(
                pages * page_size,
                page_size
            ).unwrap())
        };

        assert!(!ptr.is_null());
        ptr
    }

    fn page_size(&self) -> usize {
        4096
    }

    fn filesystem(&self, password_opt: Option<&[u8]>) -> syscall::Result<redoxfs::FileSystem<OsDisk>> {
        let disk = OsDisk::new(u8::try_from(self.boot_disk).unwrap(), self.thunk13);

        //TODO: get block from partition table
        redoxfs::FileSystem::open(disk, password_opt, None, false)
    }

    fn memory_map(&self) -> MemoryMapIter {
        MemoryMapIter::new(self.thunk15)
    }

    fn video_outputs(&self) -> usize {
        //TODO: return 1 only if vbe supported?
        1
    }

    fn video_modes(&self, _output_i: usize) -> VideoModeIter {
        VideoModeIter::new(self.thunk10)
    }

    fn set_video_mode(&self, _output_i: usize, mode: &mut OsVideoMode) {
        // Set video mode
        let mut data = ThunkData::new();
        data.eax = 0x4F02;
        data.ebx = mode.id;
        unsafe { data.with(self.thunk10); }
        //TODO: check result
    }

    fn best_resolution(&self, _output_i: usize) -> Option<(u32, u32)> {
        let mut data = ThunkData::new();
        data.eax = 0x4F15;
        data.ebx = 0x01;
        data.ecx = 0;
        data.edx = 0;
        data.edi = VBE_EDID_ADDR as u32;
        unsafe { data.with(self.thunk10); }

        if data.eax == 0x4F {
            let edid = unsafe {
                slice::from_raw_parts(VBE_EDID_ADDR as *const u8, 128)
            };

            Some((
                (edid[0x38] as u32) | (((edid[0x3A] as u32) & 0xF0) << 4),
                (edid[0x3B] as u32) | (((edid[0x3D] as u32) & 0xF0) << 4),
            ))
        } else {
            println!("Failed to get VBE EDID: 0x{:X}", { data.eax });
            None
        }
    }

    fn get_key(&self) -> OsKey {
        // Read keypress
        let mut data = ThunkData::new();
        unsafe { data.with(self.thunk16); }
        match (data.eax >> 8) as u8 {
            0x4B => OsKey::Left,
            0x4D => OsKey::Right,
            0x48 => OsKey::Up,
            0x50 => OsKey::Down,
            0x0E => OsKey::Backspace,
            0x53 => OsKey::Delete,
            0x1C => OsKey::Enter,
            _ => match data.eax as u8 {
                0 => OsKey::Other,
                b => OsKey::Char(b as char),
            }
        }
    }

    fn text_clear(&self) {
        unsafe {
            VGA.clear();
        }
    }

    fn get_text_position(&self) -> (usize, usize) {
        unsafe {
            (VGA.x, VGA.y)
        }
    }

    fn set_text_position(&self, x: usize, y: usize) {
        //TODO: ensure this is inside bounds!
        unsafe {
            VGA.x = x;
            VGA.y = y;
        }
    }

    fn set_text_color(&self, color: u8) {
        unsafe {
            VGA.color = color;
        }
    }

    fn time(&self, time: &mut AbiTime) {
        time::Time { thunk1a: self.thunk1a }.read(time)
    }

    fn delay_us(&self, microseconds: u32) {
        let mut data = ThunkData::new();
        data.eax = 0x8600;
        data.ecx = (microseconds >> 16) & 0xFFFF;
        data.edx = microseconds & 0xFFFF;
        unsafe { data.with(self.thunk15); }
    }
}

#[no_mangle]
pub unsafe extern "C" fn start(
    _kernel_entry: extern "C" fn(
        page_table: usize,
        stack: u64,
        func: u64,
        args: *const u8,
        long_mode: usize,
    ) -> !,
    boot_disk: usize,
    thunk10: extern "C" fn(),
    thunk13: extern "C" fn(),
    thunk15: extern "C" fn(),
    thunk16: extern "C" fn(),
    thunk1a: extern "C" fn(),
) -> ! {
    let os = OsBios {
        boot_disk,
        thunk10,
        thunk13,
        thunk15,
        thunk16,
        thunk1a,
    };

    #[cfg(feature = "serial_debug")]
    {
        serial::COM1.init();
        serial::COM1.write(b"SERIAL\n");
    }

    {
        // Make sure we are in mode 3 (80x25 text mode)
        let mut data = ThunkData::new();
        data.eax = 0x03;
        data.with(thunk10);
    }

    {
        // Disable cursor
        let mut data = ThunkData::new();
        data.eax = 0x0100;
        data.ecx = 0x3F00;
        data.with(thunk10);
    }

    // Clear screen
    VGA.clear();

    let (heap_start, heap_size) = memory_map(os.thunk15)
        .expect("No memory for heap");

    HEAP.init(heap_start as *mut u8, heap_size);

    crate::main(os)
}
