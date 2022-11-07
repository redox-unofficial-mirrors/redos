use redos_abi::AbiTime;

use super::ThunkData;

pub struct Time {
    pub thunk1a: extern "C" fn(),
}

fn from_bcd(bcd: u8) -> u8 {
    let high = (bcd >> 4) & 0xF;
    let low = bcd & 0xF;

    high * 10 + low
}

impl Time {
    pub fn read(&self, time: &mut AbiTime) {
        {
            let mut data = ThunkData::new();
            data.eax = 0x0400;
            unsafe { data.with(self.thunk1a) };

            time.year = from_bcd((data.ecx >> 8) as u8) as u16 * 100
                     + from_bcd(data.ecx as u8) as u16;
            time.month = from_bcd((data.edx >> 8) as u8);
            time.day = from_bcd(data.edx as u8);
        };

        {
            let mut data = ThunkData::new();
            data.eax = 0x0200;
            unsafe { data.with(self.thunk1a) };

            time.hour = from_bcd((data.ecx >> 8) as u8);
            time.minute = from_bcd(data.ecx as u8);
            time.second = from_bcd((data.edx >> 8) as u8);
            //let dst = data.edx as u8;
        };

        time.nanosecond = 0; // TODO
    }
}
