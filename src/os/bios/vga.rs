use core::{fmt, slice};

#[derive(Clone, Copy)]
#[repr(packed)]
pub struct VgaTextBlock {
    pub char: u8,
    pub color: u8,
}

pub struct Vga {
    pub base: usize,
    pub width: usize,
    pub height: usize,
    pub x: usize,
    pub y: usize,
    pub color: u8,
}

impl Vga {
    pub const unsafe fn new(base: usize, width: usize, height: usize) -> Self {
        Self {
            base,
            width,
            height,
            x: 0,
            y: 0,
            color: 0x07,
        }
    }

    pub unsafe fn blocks(&mut self) -> &'static mut [VgaTextBlock] {
        slice::from_raw_parts_mut(self.base as *mut VgaTextBlock, self.width * self.height)
    }

    pub fn clear(&mut self) {
        self.x = 0;
        self.y = 0;
        let blocks = unsafe { self.blocks() };
        for i in 0..blocks.len() {
            blocks[i] = VgaTextBlock {
                char: 0,
                color: self.color,
            };
        }
    }
}

impl fmt::Write for Vga {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        let mut blocks = unsafe { self.blocks() };
        for c in s.chars() {
            if self.x >= self.width {
                self.x = 0;
                self.y += 1;
            }
            while self.y >= self.height {
                for y in 1..self.height {
                    for x in 0..self.width {
                        let i = y * self.width + x;
                        let j = i - self.width;
                        blocks[j] = blocks[i];
                        if y + 1 == self.height {
                            blocks[i].char = 0;
                        }
                    }
                }
                self.y -= 1;
            }
            match c {
                '\x08' => {
                    if self.x > 0 {
                        self.x -= 1;
                    }
                }
                '\r' => {
                    self.x = 0;
                }
                '\n' => {
                    self.x = 0;
                    self.y += 1;
                }
                _ => {
                    let i = self.y * self.width + self.x;
                    if let Some(block) = blocks.get_mut(i) {
                        block.char = c as u8;
                        block.color = self.color;
                    }
                    self.x += 1;
                }
            }
        }

        Ok(())
    }
}
