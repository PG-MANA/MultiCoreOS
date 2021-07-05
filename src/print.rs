//! 文字列表示用モジュール
//!
//! シリアルポートと画面に文字を出力するためのモジュールとマクロを定義しています。

mod graphic;
mod serial_port;

use graphic::GraphicManager;
use serial_port::SerialPortManager;

use core::fmt;

pub static mut PRINT_MANAGER: PrintManager = PrintManager::new();

pub struct PrintManager {
    serial_port_manager: SerialPortManager,
    graphic_manager: GraphicManager,
}

impl PrintManager {
    pub const fn new() -> Self {
        Self {
            /* COM1: QEMUなどのシリアルポートタブで表示されるポート用 */
            serial_port_manager: SerialPortManager::new(0x3F8),
            graphic_manager: GraphicManager::new(),
        }
    }

    pub fn init(
        &mut self,
        frame_buffer_address: usize,
        frame_buffer_width: usize,
        frame_buffer_height: usize,
        frame_buffer_depth: u8,
        font_address: usize,
        font_size: usize,
    ) {
        self.graphic_manager.init(
            frame_buffer_address,
            frame_buffer_width,
            frame_buffer_height,
            frame_buffer_depth,
            font_address,
            font_size,
        );
    }
}

impl fmt::Write for PrintManager {
    fn write_str(&mut self, string: &str) -> fmt::Result {
        self.serial_port_manager.send_str(string);
        self.graphic_manager.draw_string(string);
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    use core::fmt::Write;
    unsafe {
        assert!(PRINT_MANAGER.write_fmt(args).is_ok());
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::print::print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt,"\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"),$($arg)*)); //\nをつける
}
