use core::arch::asm;
use hwio::{Io, Pio, WriteOnly};

pub struct Pit {
    pub chan_0: Pio<u8>,
    pub chan_1: Pio<u8>,
    pub chan_2: Pio<u8>,
    pub cmd: WriteOnly<Pio<u8>>,
}

impl Pit {
    pub const CMD_CHAN_0: u8 = 0 << 6;
    pub const CMD_CHAN_1: u8 = 1 << 6;
    pub const CMD_CHAN_2: u8 = 2 << 6;
    pub const CMD_CHAN_READ_BACK: u8 = 3 << 6;

    pub const CMD_ACCESS_LATCH: u8 = 0 << 4;
    pub const CMD_ACCESS_LOW: u8 = 1 << 4;
    pub const CMD_ACCESS_HIGH: u8 = 2 << 4;
    pub const CMD_ACCESS_LOW_HIGH: u8 = 3 << 4;

    pub const CMD_MODE_0: u8 = 0 << 1;
    pub const CMD_MODE_1: u8 = 1 << 1;
    pub const CMD_MODE_2: u8 = 2 << 1;
    pub const CMD_MODE_3: u8 = 3 << 1;
    pub const CMD_MODE_4: u8 = 4 << 1;
    pub const CMD_MODE_5: u8 = 5 << 1;

    pub const CMD_BCD: u8 = 1 << 0;

    pub const FREQUENCY: u32 = 1_193_180;
    pub const PERIOD_NS: u32 = 838;

    pub fn new() -> Self {
        Self {
            chan_0: Pio::new(0x40),
            chan_1: Pio::new(0x41),
            chan_2: Pio::new(0x42),
            cmd: WriteOnly::new(Pio::new(0x43)),
        }
    }

    pub fn chan_0_counter(&mut self) -> u16 {
        //TODO: critical code wrapper
        unsafe { asm!("pushf") };
        unsafe { asm!("cli") };
        self.cmd.write(Self::CMD_CHAN_0 | Self::CMD_ACCESS_LATCH);
        let low = self.chan_0.read();
        let high = self.chan_0.read();
        unsafe { asm!("popf") };
        ((high as u16) << 8) | (low as u16)
    }

    pub fn chan_2_freq(&mut self, freq: u32) {
        let div = Self::FREQUENCY / freq;
        //TODO: critical code wrapper
        unsafe { asm!("pushf") };
        unsafe { asm!("cli") };
        self.cmd.write(Self::CMD_CHAN_2 | Self::CMD_ACCESS_LOW_HIGH | Self::CMD_MODE_3);
        self.chan_2.write(div as u8);
        self.chan_2.write((div >> 8) as u8);
        unsafe { asm!("popf") };
    }
}
