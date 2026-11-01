//! COM1 シリアルポート (0x3F8)。`qemu -serial stdio` / `-nographic` で端末から操作できる。

use crate::port::{inb, outb};
use core::fmt;

const COM1: u16 = 0x3F8;

/// シリアルポートを初期化し、存在するかを返す。
pub fn init() -> bool {
    unsafe {
        outb(COM1 + 1, 0x00); // 割り込み無効
        outb(COM1 + 3, 0x80); // DLAB on
        outb(COM1, 0x01); // 115200 baud (divisor 1)
        outb(COM1 + 1, 0x00);
        outb(COM1 + 3, 0x03); // 8N1
        outb(COM1 + 2, 0xC7); // FIFO 有効
        outb(COM1 + 4, 0x0B); // RTS/DSR
        // ステータスが 0xFF ならポートが存在しない
        inb(COM1 + 5) != 0xFF
    }
}

pub fn write_byte(b: u8) {
    unsafe {
        let mut spins = 0u32;
        while inb(COM1 + 5) & 0x20 == 0 {
            spins += 1;
            if spins > 100_000 {
                return;
            }
        }
        outb(COM1, b);
    }
}

pub fn read_byte() -> Option<u8> {
    unsafe {
        let status = inb(COM1 + 5);
        if status != 0xFF && status & 0x01 != 0 {
            Some(inb(COM1))
        } else {
            None
        }
    }
}

/// panic 時など、どこからでも使える書き込み用ハンドル。
pub struct SerialWriter;

impl fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            if b == b'\n' {
                write_byte(b'\r');
            }
            write_byte(b);
        }
        Ok(())
    }
}
