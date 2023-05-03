use alloc::{string::String, vec::Vec};
use hwio::{Io, Pio, ReadOnly, WriteOnly};

use super::isadma::IsaDmaChannel;

#[allow(dead_code)]
pub struct Sb16 {
    pub version: (u8, u8),
    pub irqs: Vec<u8>,
    pub dmas: Vec<IsaDmaChannel>,
    // Regs
    /* 0x04 */ mixer_addr: WriteOnly<Pio<u8>>,
    /* 0x05 */ mixer_data: Pio<u8>,
    /* 0x06 */ dsp_reset: WriteOnly<Pio<u8>>,
    /* 0x0A */ dsp_read_data: ReadOnly<Pio<u8>>,
    /* 0x0C */ dsp_write_data: WriteOnly<Pio<u8>>,
    /* 0x0C */ dsp_write_status: ReadOnly<Pio<u8>>,
    /* 0x0E */ dsp_read_status: ReadOnly<Pio<u8>>,
}

impl Sb16 {
    pub unsafe fn new<D: Fn(u32)>(addr: u16, delay_us: D) -> Result<Self, String> {
        let mut module = Sb16 {
            version: (0, 0),
            irqs: Vec::new(),
            dmas: Vec::new(),
            // Regs
            mixer_addr: WriteOnly::new(Pio::new(addr + 0x04)),
            mixer_data: Pio::new(addr + 0x05),
            dsp_reset: WriteOnly::new(Pio::new(addr + 0x06)),
            dsp_read_data: ReadOnly::new(Pio::new(addr + 0x0A)),
            dsp_write_data: WriteOnly::new(Pio::new(addr + 0x0C)),
            dsp_write_status: ReadOnly::new(Pio::new(addr + 0x0C)),
            dsp_read_status: ReadOnly::new(Pio::new(addr + 0x0E)),
        };

        module.init(delay_us)?;

        Ok(module)
    }

    fn mixer_read(&mut self, index: u8) -> u8 {
        self.mixer_addr.write(index);
        self.mixer_data.read()
    }

    fn mixer_write(&mut self, index: u8, value: u8) {
        self.mixer_addr.write(index);
        self.mixer_data.write(value);
    }

    fn dsp_read(&mut self) -> Result<u8, String> {
        // Bit 7 must be 1 before data can be sent
        while !self.dsp_read_status.readf(1 << 7) {
            //TODO: timeout!
        }

        Ok(self.dsp_read_data.read())
    }

    fn dsp_write(&mut self, value: u8) -> Result<(), String> {
        // Bit 7 must be 0 before data can be sent
        while self.dsp_write_status.readf(1 << 7) {
            //TODO: timeout!
        }

        self.dsp_write_data.write(value);
        Ok(())
    }

    fn init<D: Fn(u32)>(&mut self, delay_us: D) -> Result<(), String> {
        // Perform DSP reset
        {
            // Write 1 to reset port
            self.dsp_reset.write(1);

            // Wait 3us
            delay_us(3);

            // Write 0 to reset port
            self.dsp_reset.write(0);

            //TODO: Wait for ready byte (0xAA) using read status
            delay_us(100);

            let ready = self.dsp_read()?;
            if ready != 0xAA {
                return Err(format!("ready byte was 0x{:02X} instead of 0xAA", ready));
            }
        }

        // Read DSP version
        {
            self.dsp_write(0xE1)?;

            let major = self.dsp_read()?;
            let minor = self.dsp_read()?;
            self.version = (major, minor);

            if major != 4 {
                return Err(format!("Unsupported DSP major version {}", major));
            }
        }

        // Get available IRQs and DMAs
        {
            self.irqs.clear();
            let irq_mask = self.mixer_read(0x80);
            if (irq_mask & (1 << 0)) != 0 {
                self.irqs.push(2);
            }
            if (irq_mask & (1 << 1)) != 0 {
                self.irqs.push(5);
            }
            if (irq_mask & (1 << 2)) != 0 {
                self.irqs.push(7);
            }
            if (irq_mask & (1 << 3)) != 0 {
                self.irqs.push(10);
            }

            self.dmas.clear();
            let dma_mask = self.mixer_read(0x81);
            if (dma_mask & (1 << 0)) != 0 {
                self.dmas.push(IsaDmaChannel::Zero);
            }
            if (dma_mask & (1 << 1)) != 0 {
                self.dmas.push(IsaDmaChannel::One);
            }
            if (dma_mask & (1 << 3)) != 0 {
                self.dmas.push(IsaDmaChannel::Three);
            }
            if (dma_mask & (1 << 5)) != 0 {
                self.dmas.push(IsaDmaChannel::Five);
            }
            if (dma_mask & (1 << 6)) != 0 {
                self.dmas.push(IsaDmaChannel::Six);
            }
            if (dma_mask & (1 << 7)) != 0 {
                self.dmas.push(IsaDmaChannel::Seven);
            }
        }

        // Set output sample rate to 44100 Hz (Redox OS standard)
        {
            let rate = 44100u16;
            self.dsp_write(0x41)?;
            self.dsp_write((rate >> 8) as u8)?;
            self.dsp_write(rate as u8)?;
        }

        Ok(())
    }
}
