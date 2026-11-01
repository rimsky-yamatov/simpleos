//! VGA + シリアル + PS/2 キーボードをまとめたカーネルのコンソール。

use alloc::{collections::VecDeque, string::String};
use basic_core::Console;

use crate::keyboard::Keyboard;
use crate::serial;
use crate::vga::Vga;

pub struct KernelConsole {
    vga: Vga,
    keyboard: Keyboard,
    serial: bool,
    pending: VecDeque<u8>,
}

impl KernelConsole {
    pub fn new() -> Self {
        let serial = serial::init();
        let mut vga = Vga::new();
        vga.clear();
        Self {
            vga,
            keyboard: Keyboard::new(),
            serial,
            pending: VecDeque::new(),
        }
    }

    fn put_byte(&mut self, b: u8) {
        self.vga.write_byte(b);
        if self.serial {
            match b {
                b'\n' => {
                    serial::write_byte(b'\r');
                    serial::write_byte(b'\n');
                }
                0x08 => {
                    // 端末側でも1文字消す
                    serial::write_byte(0x08);
                    serial::write_byte(b' ');
                    serial::write_byte(0x08);
                }
                _ => serial::write_byte(b),
            }
        }
    }

    pub fn print(&mut self, s: &str) {
        for b in s.bytes() {
            self.put_byte(b);
        }
    }

    pub fn clear(&mut self) {
        self.vga.clear();
        if self.serial {
            self.print_serial_only("\x1b[2J\x1b[H");
        }
    }

    fn print_serial_only(&mut self, s: &str) {
        for b in s.bytes() {
            serial::write_byte(b);
        }
    }

    /// キーボードかシリアルから 1 バイト取得(無ければ None)。
    fn poll_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.keyboard.poll() {
            return Some(b);
        }
        if self.serial {
            if let Some(b) = serial::read_byte() {
                return Some(match b {
                    b'\r' => b'\n',
                    0x7F => 0x08,
                    other => other,
                });
            }
        }
        None
    }

    fn next_byte(&mut self) -> u8 {
        loop {
            if let Some(b) = self.pending.pop_front() {
                return b;
            }
            if let Some(b) = self.poll_byte() {
                return b;
            }
            core::hint::spin_loop();
        }
    }

    /// 1 行入力する(エコーバック・BackSpace 対応)。
    pub fn input_line(&mut self) -> String {
        let mut buf = String::new();
        loop {
            let b = self.next_byte();
            match b {
                b'\n' => {
                    self.put_byte(b'\n');
                    return buf;
                }
                0x08 => {
                    if buf.pop().is_some() {
                        self.put_byte(0x08);
                    }
                }
                3 => {
                    // Ctrl-C: 入力中の行を捨てる
                    self.print("^C\n");
                    return String::new();
                }
                0x20..=0x7E => {
                    if buf.len() < 250 {
                        buf.push(b as char);
                        self.put_byte(b);
                    }
                }
                _ => {}
            }
        }
    }
}

impl Console for KernelConsole {
    fn read_line(&mut self) -> Option<String> {
        Some(self.input_line())
    }

    fn write_line(&mut self, s: &str) {
        self.print(s);
        self.put_byte(b'\n');
    }

    fn write_prompt(&mut self, s: &str) {
        self.print(s);
    }

    fn interrupted(&mut self) -> bool {
        // 実行中に押されたキーを調べ、Ctrl-C / Esc なら中断。他は先行入力として保持する
        while let Some(b) = self.poll_byte() {
            if b == 3 || b == 27 {
                self.pending.clear();
                return true;
            }
            if self.pending.len() < 256 {
                self.pending.push_back(b);
            }
        }
        false
    }

    fn clear_screen(&mut self) {
        self.clear();
    }
}
