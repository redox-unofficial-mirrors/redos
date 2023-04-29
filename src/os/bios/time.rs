use redos_abi::AbiTime;
use redos_hw::pit::Pit;

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

        let year = from_bcd((data.ecx >> 8) as u8) as u16 * 100
                 + from_bcd(data.ecx as u8) as u16;
        let month = from_bcd((data.edx >> 8) as u8);
        let day = from_bcd(data.edx as u8);
        (year, month, day)
    }

    //TODO: why is this skipping time?
    pub fn read(&self, time: &mut AbiTime) {
        let mut pit = Pit::new();

        loop {
            // Read PIT counter
            let start_counter = pit.chan_0_counter();

            // Read BIOS system time, 18.2 ticks per second
            let start_ticks = self.system_time();

            // Read date from RTC using BIOS INT 1Ah, AH=04h
            let start_date = self.rtc_date();

            // Read date again
            let date = self.rtc_date();

            // Read BIOS system time again
            let ticks = self.system_time();

            // Read PIT counter again
            let counter = pit.chan_0_counter();

            // If date rollover, loop
            if start_date != date {
                continue;
            }

            // If time rollover, loop
            if start_ticks != ticks {
                continue;
            }

            // Check for rollover
            if counter > start_counter {
                continue;
            }

            // Set date
            time.year = date.0;
            time.month = date.1;
            time.day = date.2;

            // Set time
            let ticks_10 = ticks * 10;
            let seconds = ticks_10 / 182;
            let minutes = seconds / 60;
            let hours = minutes / 60;
            time.hour = (hours % 24) as u8;
            time.minute = (minutes % 60) as u8;
            time.second = (seconds % 60) as u8;
            time.nanosecond = (ticks_10 % 182) * (1_000_000_000 / 182);

            // Add remaining PIT counter
            time.nanosecond += (0xFFFFu16.saturating_sub(counter) as u32) * (Pit::PERIOD_NS as u32);

            break;
        }
    }
}
