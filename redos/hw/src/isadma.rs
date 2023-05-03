use hwio::{Pio, ReadOnly, WriteOnly};

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
    channel: IsaDmaChannel,
    primary: bool,
    index: u8,
    addr: WriteOnly<Pio<u16>>,
    count: WriteOnly<Pio<u16>>,
    status: ReadOnly<Pio<u8>>,
    command: WriteOnly<Pio<u8>>,
    request: WriteOnly<Pio<u8>>,
    mask: WriteOnly<Pio<u8>>,
    mode: WriteOnly<Pio<u8>>,
    flip_flop: WriteOnly<Pio<u8>>,
}

impl IsaDma {
    pub fn new(channel: IsaDmaChannel) -> Self {
        let (primary, index) = match channel {
            IsaDmaChannel::Zero => (true, 0),
            IsaDmaChannel::One => (true, 1),
            IsaDmaChannel::Two => (true, 2),
            IsaDmaChannel::Three => (true, 3),
            IsaDmaChannel::Four => (false, 0),
            IsaDmaChannel::Five => (false, 1),
            IsaDmaChannel::Six => (false, 2),
            IsaDmaChannel::Seven => (false, 3),
        };

        Self {
            channel,
            primary,
            index,
            addr: WriteOnly::new(Pio::new(if primary {
                (0x00 + index).into()
            } else {
                (0xC0 + index * 2).into()
            })),
            count: WriteOnly::new(Pio::new(if primary {
                (0x01 + index).into()
            } else {
                (0xC2 + index * 2).into()
            })),
            status: ReadOnly::new(Pio::new(if primary { 0x08 } else { 0xD0 })),
            command: WriteOnly::new(Pio::new(if primary { 0x08 } else { 0xD0 })),
            request: WriteOnly::new(Pio::new(if primary { 0x09 } else { 0xD2 })),
            mask: WriteOnly::new(Pio::new(if primary { 0x0A } else { 0xD4 })),
            mode: WriteOnly::new(Pio::new(if primary { 0x0B } else { 0xD6 })),
            flip_flop: WriteOnly::new(Pio::new(if primary { 0x0C } else { 0xD8 })),
        }
    }
}
