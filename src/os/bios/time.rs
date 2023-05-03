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
    // Read system time (BIOS counter using IRQ0) using BIOS INT 1Ah, AH=00h
    // BIOS system time is defined as PIT overflows since midnight
    //TODO: why is this broke?
    fn system_time(&self) -> u32 {
        let mut data = ThunkData::new();
        data.eax = 0x0000;
        unsafe { data.with(self.thunk1a) };

        ((data.ecx & 0xFFFF) << 16) | (data.edx & 0xFFFF)
    }

    // Read date from RTC using BIOS INT 1Ah, AH=04h
    //TODO: use RTC registers
    fn rtc_date(&self) -> (u16, u8, u8) {
        let mut data = ThunkData::new();
        data.eax = 0x0400;
        unsafe { data.with(self.thunk1a) };

        let year = from_bcd((data.ecx >> 8) as u8) as u16 * 100 + from_bcd(data.ecx as u8) as u16;
        let month = from_bcd((data.edx >> 8) as u8);
        let day = from_bcd(data.edx as u8);
        (year, month, day)
    }

    // Read time from RTC using BIOS INT 1Ah, AH=02h
    //TODO: use RTC registers
    fn rtc_time(&self) -> (u8, u8, u8) {
        let mut data = ThunkData::new();
        data.eax = 0x0200;
        unsafe { data.with(self.thunk1a) };

        let hour = from_bcd((data.ecx >> 8) as u8);
        let minute = from_bcd(data.ecx as u8);
        let second = from_bcd((data.edx >> 8) as u8);
        (hour, minute, second)
    }

    //TODO: why is this skipping time?
    pub fn read(&self, abi_time: &mut AbiTime) {
        loop {
            // Read time from RTC using BIOS INT 1Ah, AH=02h
            let start_time = self.rtc_time();

            // Read date from RTC using BIOS INT 1Ah, AH=04h
            let start_date = self.rtc_date();

            // Read date again
            let date = self.rtc_date();

            // Read time again
            let time = self.rtc_time();

            // If date rollover, loop
            if start_date != date {
                continue;
            }

            // If time rollover, loop
            if start_time != time {
                continue;
            }

            // Set date
            abi_time.year = date.0;
            abi_time.month = date.1;
            abi_time.day = date.2;

            // Set time
            abi_time.hour = time.0;
            abi_time.minute = time.1;
            abi_time.second = time.2;
            //TODO: can we get this again using PIT?
            abi_time.nanosecond = 0;

            break;
        }
    }
}
