use core::arch::asm;
use hwio::{Io, Pio};

pub struct PcSpeaker {
    port: Pio<u8>,
}

impl PcSpeaker {
    const TIMER: u8 = 1 << 0;
    const DATA: u8 = 1 << 1;

    pub fn new() -> Self {
        Self {
            port: Pio::new(0x61),
        }
    }

    pub fn start(&mut self) {
        self.port.writef(Self::TIMER | Self::DATA, true);
    }

    pub fn stop(&mut self) {
        self.port.writef(Self::TIMER | Self::DATA, false);
    }
}
