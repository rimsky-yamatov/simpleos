//! VGA テキストモード (80x25, 0xB8000) の出力。

use crate::port::outb;
use core::fmt;
use core::ptr::{read_volatile, write_volatile};

const BUFFER: *mut u16 = 0xB8000 as *mut u16;
const WIDTH: usize = 80;
const HEIGHT: usize = 25;

pub struct Vga {
    row: usize,
    col: usize,
    color: u8,
}

impl Vga {
    pub const fn new() -> Self {
        Self { row: 0, col: 0, color: 0x0F }
    }

    pub fn set_color(&mut self, color: u8) {
        self.color = color;
    }

    fn cell(&self, ch: u8) -> u16 {
        ((self.color as u16) << 8) | ch as u16
    }

    fn put(&self, row: usize, col: usize, ch: u8) {
        unsafe {
            write_volatile(BUFFER.add(row * WIDTH + col), self.cell(ch));
        }
    }

    pub fn clear(&mut self) {
        for r in 0..HEIGHT {
            for c in 0..WIDTH {
                self.put(r, c, b' ');
            }
        }
        self.row = 0;
        self.col = 0;
        self.update_cursor();
    }

    fn scroll(&mut self) {
        unsafe {
            for r in 1..HEIGHT {
                for c in 0..WIDTH {
                    let v = read_volatile(BUFFER.add(r * WIDTH + c));
                    write_volatile(BUFFER.add((r - 1) * WIDTH + c), v);
                }
            }
        }
        for c in 0..WIDTH {
            self.put(HEIGHT - 1, c, b' ');
        }
    }

    fn newline(&mut self) {
        self.col = 0;
        if self.row + 1 < HEIGHT {
            self.row += 1;
        } else {
            self.scroll();
        }
    }

    pub fn write_byte(&mut self, b: u8) {
        match b {
            b'\n' => self.newline(),
            b'\r' => self.col = 0,
            0x08 => {
                if self.col > 0 {
                    self.col -= 1;
                } else if self.row > 0 {
                    self.row -= 1;
                    self.col = WIDTH - 1;
                } else {
                    return;
                }
                self.put(self.row, self.col, b' ');
            }
            b'\t' => {
                let next = (self.col / 8 + 1) * 8;
                while self.col < next {
                    self.write_byte(b' ');
                    if self.col == 0 {
                        break; // 行が折り返したら終わり
                    }
                }
            }
            0x20..=0x7E => {
                self.put(self.row, self.col, b);
                self.col += 1;
                if self.col >= WIDTH {
                    self.newline();
                }
            }
            _ => {
                self.put(self.row, self.col, b'?');
                self.col += 1;
                if self.col >= WIDTH {
                    self.newline();
                }
            }
        }
        self.update_cursor();
    }

    pub fn write_str_raw(&mut self, s: &str) {
        for b in s.bytes() {
            self.write_byte(b);
        }
    }

    fn update_cursor(&self) {
        let pos = (self.row * WIDTH + self.col) as u16;
        unsafe {
            outb(0x3D4, 0x0F);
            outb(0x3D5, (pos & 0xFF) as u8);
            outb(0x3D4, 0x0E);
            outb(0x3D5, (pos >> 8) as u8);
        }
    }
}

impl fmt::Write for Vga {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str_raw(s);
        Ok(())
    }
}
