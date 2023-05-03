use alloc::alloc::{alloc_zeroed, dealloc, Layout};
use core::{
    ops::{Deref, DerefMut},
    slice,
};
use hwio::{Pio, ReadOnly, WriteOnly};

#[derive(Clone, Copy, Debug)]
pub enum IsaDmaChannel {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
}

pub struct IsaDma {
    pub channel: IsaDmaChannel,
    pub primary: bool,
    pub index: u8,
    pub page: Pio<u8>,
    pub address: WriteOnly<Pio<u8>>,
    pub count: WriteOnly<Pio<u8>>,
    pub status: ReadOnly<Pio<u8>>,
    pub command: WriteOnly<Pio<u8>>,
    pub request: WriteOnly<Pio<u8>>,
    pub mask: WriteOnly<Pio<u8>>,
    pub mode: WriteOnly<Pio<u8>>,
    pub flip_flop: WriteOnly<Pio<u8>>,
}

impl IsaDma {
    pub fn new(channel: IsaDmaChannel) -> Self {
        let (primary, index, page_port, address_port, count_port) = match channel {
            IsaDmaChannel::Zero => (true, 0, 0x87, 0x00, 0x01),
            IsaDmaChannel::One => (true, 1, 0x83, 0x02, 0x03),
            IsaDmaChannel::Two => (true, 2, 0x81, 0x04, 0x05),
            IsaDmaChannel::Three => (true, 3, 0x82, 0x06, 0x07),
            IsaDmaChannel::Four => (false, 0, 0x8F, 0xC0, 0xC2),
            IsaDmaChannel::Five => (false, 1, 0x8B, 0xC4, 0xC6),
            IsaDmaChannel::Six => (false, 2, 0x89, 0xC8, 0xCA),
            IsaDmaChannel::Seven => (false, 3, 0x8A, 0xCC, 0xCE),
        };

        Self {
            channel,
            primary,
            index,
            page: Pio::new(page_port),
            address: WriteOnly::new(Pio::new(address_port)),
            count: WriteOnly::new(Pio::new(count_port)),
            status: ReadOnly::new(Pio::new(if primary { 0x08 } else { 0xD0 })),
            command: WriteOnly::new(Pio::new(if primary { 0x08 } else { 0xD0 })),
            request: WriteOnly::new(Pio::new(if primary { 0x09 } else { 0xD2 })),
            mask: WriteOnly::new(Pio::new(if primary { 0x0A } else { 0xD4 })),
            mode: WriteOnly::new(Pio::new(if primary { 0x0B } else { 0xD6 })),
            flip_flop: WriteOnly::new(Pio::new(if primary { 0x0C } else { 0xD8 })),
        }
    }
}

pub struct IsaDmaBuffer {
    ptr: *mut u8,
    layout: Layout,
}

impl IsaDmaBuffer {
    pub fn new() -> Option<Self> {
        let layout = Layout::from_size_align(0x1_0000, 0x1_0000).unwrap();
        let ptr = unsafe { alloc_zeroed(layout) };
        assert!(!ptr.is_null());
        let buffer = Self { ptr, layout };
        if buffer.address() < 0x1_00_0000 {
            Some(buffer)
        } else {
            None
        }
    }

    pub fn address(&self) -> usize {
        self.ptr as usize
    }
}

impl Deref for IsaDmaBuffer {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr, self.layout.size()) }
    }
}

impl DerefMut for IsaDmaBuffer {
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.layout.size()) }
    }
}

impl Drop for IsaDmaBuffer {
    fn drop(&mut self) {
        unsafe { dealloc(self.ptr, self.layout) }
    }
}
