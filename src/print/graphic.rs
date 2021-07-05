//! フレームバッファでの文字列表示用モジュール
//!
//! GRUB2で使われているpffｐ2フォントを使用します。
//! コード簡略化のため、ASCIIコード以外は対応していません。

pub struct GraphicManager {
    frame_buffer_address: usize,
    frame_buffer_width: usize,
    frame_buffer_height: usize,
    frame_buffer_depth_byte: u8,
    max_font_height: u16,
    ascent: u16,
    cursor_x: usize,
    cursor_y: usize,
    ascii_font_cache: [BitmapFontData; 0x7f - 0x20],
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct BitmapFontData {
    pub width: u16,
    pub height: u16,
    pub x_offset: i16,
    pub y_offset: i16,
    pub device_width: i16,
    pub bitmap_address: usize,
}

#[repr(C, packed)]
struct Pff2CharIndex {
    code: [u8; 4],
    flags: u8,
    offset: [u8; 4],
}

#[repr(C, packed)]
struct Pff2FontData {
    width: [u8; 2],
    height: [u8; 2],
    x_offset: [u8; 2],
    y_offset: [u8; 2],
    device_width: [u8; 2],
    bitmap: u8,
}

/// 文字色
const FRONT_COLOR: u32 = 0x55ffff;
/// 背景色
const BACK_COLOR: u32 = 0;

impl GraphicManager {
    pub const fn new() -> Self {
        let invalid_font_data = BitmapFontData {
            width: 0,
            height: 0,
            x_offset: 0,
            y_offset: 0,
            device_width: 0,
            bitmap_address: 0,
        };
        Self {
            frame_buffer_address: 0,
            frame_buffer_width: 0,
            frame_buffer_height: 0,
            frame_buffer_depth_byte: 0,
            max_font_height: 0,
            ascent: 0,
            cursor_x: 0,
            cursor_y: 0,
            ascii_font_cache: [invalid_font_data; 0x7f - 0x20],
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
        /* Load PFF2 data */
        /* Check the file structure */
        if font_address == 0
            || unsafe { *(font_address as *const [u8; 12]) }
                != [
                    0x46, 0x49, 0x4c, 0x45, 0x00, 0x00, 0x00, 0x04, 0x50, 0x46, 0x46, 0x32,
                ]
        /* "FILE PFF2" */
        {
            return;
        }

        let mut pointer = 12;
        let mut font_char_index_address = 0;
        let mut font_char_index_size = 0;

        while pointer < font_size {
            use core::{str, u16, u32};

            let section_type =
                str::from_utf8(unsafe { &*((font_address + pointer) as *const [u8; 4]) })
                    .unwrap_or("");
            let section_length =
                u32::from_be_bytes(unsafe { *((font_address + pointer + 4) as *const [u8; 4]) })
                    as usize;
            pointer += 8;

            match section_type {
                "NAME" | "FAMI" | "WEIG" | "SLAN" | "PTSZ" | "MAXW" | "DESC" => {}
                "MAXH" => {
                    self.max_font_height = u16::from_be_bytes(unsafe {
                        *((font_address + pointer) as *const [u8; 2])
                    });
                }
                "ASCE" => {
                    self.ascent = u16::from_be_bytes(unsafe {
                        *((font_address + pointer) as *const [u8; 2])
                    });
                }
                "CHIX" => {
                    font_char_index_address = font_address + pointer;
                    font_char_index_size = section_length;
                }
                "DATA" => {
                    break;
                }
                _ => {
                    return;
                }
            };
            pointer += section_length;
        }

        if font_char_index_address == 0 {
            return;
        }
        /* Build ASCII font cache */
        pointer = 0;
        for a in ' '..'\x7f' {
            let char_utf32 = [0, 0, 0, a as u8];
            let char_index = {
                let next_entry =
                    unsafe { &*((font_char_index_address + pointer) as *const Pff2CharIndex) };
                if next_entry.code == char_utf32 {
                    next_entry
                } else {
                    pointer = 0;
                    let mut entry;
                    loop {
                        entry = unsafe {
                            &*((font_char_index_address + pointer) as *const Pff2CharIndex)
                        };
                        if entry.code == char_utf32 {
                            break;
                        }
                        pointer += core::mem::size_of::<Pff2CharIndex>();
                        if pointer >= font_char_index_size {
                            return;
                        }
                    }
                    entry
                }
            };

            let pff2_font_data = unsafe {
                &*((u32::from_be_bytes(char_index.offset) as usize + font_address)
                    as *const Pff2FontData)
            };
            let font_data = BitmapFontData {
                width: u16::from_be_bytes(pff2_font_data.width),
                height: u16::from_be_bytes(pff2_font_data.height),
                x_offset: i16::from_be_bytes(pff2_font_data.x_offset),
                y_offset: i16::from_be_bytes(pff2_font_data.y_offset),
                device_width: i16::from_be_bytes(pff2_font_data.device_width),
                bitmap_address: &pff2_font_data.bitmap as *const u8 as usize,
            };
            self.ascii_font_cache[a as usize - 0x20] = font_data;
            pointer += core::mem::size_of::<Pff2CharIndex>();
        }

        self.frame_buffer_address = frame_buffer_address;
        self.frame_buffer_width = frame_buffer_width;
        self.frame_buffer_height = frame_buffer_height;
        self.frame_buffer_depth_byte = frame_buffer_depth >> 3;
    }

    fn write_font_bitmap(
        &mut self,
        bitmap_address: usize,
        size_x: usize,
        size_y: usize,
        offset_x: usize,
        offset_y: usize,
    ) {
        let mut bitmap_pointer = bitmap_address;
        let mut bitmap_mask = 0x80;
        let mut buffer_pointer = self.frame_buffer_address
            + (offset_y * self.frame_buffer_width + offset_x)
                * self.frame_buffer_depth_byte as usize;

        if self.frame_buffer_depth_byte == 4 {
            for _ in 0..size_y {
                for _ in 0..size_x {
                    unsafe {
                        *(buffer_pointer as *mut u32) =
                            if (*(bitmap_pointer as *const u8) & bitmap_mask) != 0 {
                                FRONT_COLOR
                            } else {
                                BACK_COLOR
                            }
                    };
                    buffer_pointer += self.frame_buffer_depth_byte as usize;
                    bitmap_mask >>= 1;
                    if bitmap_mask == 0 {
                        bitmap_pointer += 1;
                        bitmap_mask = 0x80;
                    }
                }
                buffer_pointer +=
                    (self.frame_buffer_width - size_x) * self.frame_buffer_depth_byte as usize;
            }
        } else {
            for _ in 0..size_y {
                for _ in 0..size_x {
                    let dot = buffer_pointer as *mut u32;
                    unsafe {
                        *dot &= 0x000000ff;
                        *dot |= if (*(bitmap_pointer as *const u8) & bitmap_mask) != 0 {
                            FRONT_COLOR
                        } else {
                            BACK_COLOR
                        } & 0xffffff;
                    }
                    buffer_pointer += self.frame_buffer_depth_byte as usize;
                    bitmap_mask >>= 1;
                    if bitmap_mask == 0 {
                        bitmap_pointer += 1;
                        bitmap_mask = 0x80;
                    }
                }
                buffer_pointer +=
                    (self.frame_buffer_width - size_x) * self.frame_buffer_depth_byte as usize;
            }
        }
    }

    fn scroll_screen(&self, scroll_height: usize) {
        unsafe {
            core::ptr::copy(
                (self.frame_buffer_address
                    + scroll_height
                        * self.frame_buffer_width
                        * self.frame_buffer_depth_byte as usize) as *const u32,
                self.frame_buffer_address as *mut u32,
                (self.frame_buffer_height - scroll_height)
                    * self.frame_buffer_width
                    * self.frame_buffer_depth_byte as usize,
            );
            core::ptr::write_bytes(
                (self.frame_buffer_address
                    + (self.frame_buffer_height - scroll_height)
                        * self.frame_buffer_width
                        * self.frame_buffer_depth_byte as usize) as *mut u8,
                0,
                scroll_height * self.frame_buffer_width * self.frame_buffer_depth_byte as usize,
            );
        }
    }

    pub fn draw_string(&mut self, s: &str) -> bool {
        if self.frame_buffer_address == 0 {
            return false;
        }
        for c in s.chars().into_iter() {
            if c == '\n' {
                self.cursor_x = 0;
                self.cursor_y += self.max_font_height as usize;
            } else if c == '\r' {
                self.cursor_x = 0;
            } else if c.is_ascii() {
                let font_data = self.ascii_font_cache[c as usize - 0x20];
                let font_bottom = self.ascent as isize - font_data.y_offset as isize;
                let font_top = font_bottom as usize - font_data.height as usize;
                let font_left = font_data.x_offset as usize;
                if self.frame_buffer_width <= self.cursor_x + font_data.width as usize {
                    self.cursor_x = 0;
                    self.cursor_y += self.max_font_height as usize;
                }
                if self.frame_buffer_height <= self.cursor_y + font_data.height as usize {
                    let scroll_y =
                        self.max_font_height as usize + self.cursor_y - self.frame_buffer_height;
                    self.scroll_screen(scroll_y);
                    self.cursor_y -= scroll_y;
                }

                self.write_font_bitmap(
                    font_data.bitmap_address,
                    font_data.width as usize,
                    font_data.height as usize,
                    self.cursor_x + font_left,
                    self.cursor_y + font_top,
                );
                self.cursor_x += font_data.device_width as usize;
            }
        }
        return true;
    }
}
