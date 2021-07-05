//! ACPI PM Timer
//!
//! ACPIをサポートしているPCに搭載されている周波数3579545Hzのカウントアップタイマーです。
//! 今回はコアの初期化の際のビジーウェイトに使用してます。

pub struct AcpiPmTimer {
    port: usize,
    is_32_bit_counter: bool,
}

impl AcpiPmTimer {
    const FREQUENCY_HZ: usize = 3579545;
    pub const fn new(port: usize, is_32_bit_counter: bool) -> Self {
        Self {
            port,
            is_32_bit_counter,
        }
    }

    pub const fn const_new() -> Self {
        Self::new(0, false)
    }

    fn get_count(&self) -> usize {
        if self.port == 0 {
            return 0;
        }
        let mut result: u32;
        unsafe { asm!("in eax, dx", in("dx") self.port, out("eax") result) };
        if self.is_32_bit_counter == false {
            result &= 0xffffff;
        }
        result as usize
    }

    fn get_ending_count_value(&self, start: usize, difference: usize) -> usize {
        let (result, overflow) = start.overflowing_add(difference);

        if self.is_32_bit_counter {
            result
        } else if overflow == false {
            if result <= 0xffffff {
                result
            } else {
                result - 0xffffff
            }
        } else {
            result + (0xffffffff - 0xffffff)
        }
    }

    fn get_max_counter_value(&self) -> usize {
        if self.is_32_bit_counter {
            0xffffffff
        } else {
            0xffffff
        }
    }

    #[inline(always)]
    pub fn busy_wait_ms(&self, ms: usize) {
        let start = self.get_count();
        let difference = Self::FREQUENCY_HZ * ms / 1000;
        if difference > self.get_max_counter_value() {
            panic!("Cannot count more than max_counter_value");
        }

        let end = self.get_ending_count_value(start, difference);
        while self.get_count() < end {}
    }

    #[inline(always)]
    pub fn busy_wait_us(&self, us: usize) {
        let start = self.get_count();
        let difference = Self::FREQUENCY_HZ * us / 1000000;
        if difference > self.get_max_counter_value() {
            panic!("Cannot count more than max_counter_value");
        } else if difference == 0 {
            panic!("Cannot count less than the resolution");
        }
        let end = self.get_ending_count_value(start, difference);

        while self.get_count() < end {}
    }
}
