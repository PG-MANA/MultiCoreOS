//! シリアルポートでの文字列送信用モジュール
//!
//! 送信専用のモジュールです。受信のための初期化などは一切省いております。
//! また簡略化のため、ロック機構なども省略しています。

use core::arch::asm;

pub struct SerialPortManager {
    port: u16,
}

impl SerialPortManager {
    pub const fn new(io_port: u16) -> Self {
        Self { port: io_port }
    }

    pub fn send(&self, data: u8) {
        if self.port == 0 {
            return;
        }
        let mut timeout = 0xFF;
        while timeout > 0 {
            if self.is_completed_transmitter() {
                break;
            }
            timeout -= 1;
        }
        unsafe {
            asm!("out dx, al",in("dx") self.port, in("al") data);
        }
    }

    pub fn send_str(&self, s: &str) {
        for c in s.bytes() {
            if c as char == '\n' {
                self.send('\r' as u8);
            }
            self.send(c);
        }
    }

    #[inline]
    fn is_completed_transmitter(&self) -> bool {
        let mut result: u8;
        unsafe { asm!("in al, dx",in("dx") self.port + 5,out("al") result) };
        (result & 0x40) != 0
    }
}
