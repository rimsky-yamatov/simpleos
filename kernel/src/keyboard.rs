//! PS/2 キーボード(スキャンコードセット1)のポーリング入力。US 配列。

use crate::port::inb;

const DATA_PORT: u16 = 0x60;
const STATUS_PORT: u16 = 0x64;

pub struct Keyboard {
    shift: bool,
    ctrl: bool,
    caps: bool,
    extended: bool,
}

const ROW_NUM_NORMAL: &[u8] = b"1234567890-=";
const ROW_NUM_SHIFT: &[u8] = b"!@#$%^&*()_+";
const ROW_Q: &[u8] = b"qwertyuiop";
const ROW_A: &[u8] = b"asdfghjkl";
const ROW_Z: &[u8] = b"zxcvbnm";

/// スキャンコード -> ASCII。対応なしは 0。
fn map_scancode(code: u8, shift: bool) -> u8 {
    let c = code as usize;
    match code {
        0x01 => 27,
        0x02..=0x0D => {
            if shift {
                ROW_NUM_SHIFT[c - 0x02]
            } else {
                ROW_NUM_NORMAL[c - 0x02]
            }
        }
        0x0E => 8,
        0x0F => b'\t',
        0x10..=0x19 => letter(ROW_Q[c - 0x10], shift),
        0x1A => if shift { b'{' } else { b'[' },
        0x1B => if shift { b'}' } else { b']' },
        0x1C => b'\n',
        0x1E..=0x26 => letter(ROW_A[c - 0x1E], shift),
        0x27 => if shift { b':' } else { b';' },
        0x28 => if shift { b'"' } else { b'\'' },
        0x29 => if shift { b'~' } else { b'`' },
        0x2B => if shift { b'|' } else { b'\\' },
        0x2C..=0x32 => letter(ROW_Z[c - 0x2C], shift),
        0x33 => if shift { b'<' } else { b',' },
        0x34 => if shift { b'>' } else { b'.' },
        0x35 => if shift { b'?' } else { b'/' },
        0x37 => b'*',
        0x39 => b' ',
        _ => 0,
    }
}

fn letter(base: u8, shift: bool) -> u8 {
    if shift {
        base.to_ascii_uppercase()
    } else {
        base
    }
}

impl Keyboard {
    pub const fn new() -> Self {
        Self { shift: false, ctrl: false, caps: false, extended: false }
    }

    /// 入力バッファに何かあれば読んで ASCII に変換する(無ければ None)。
    pub fn poll(&mut self) -> Option<u8> {
        let (status, scancode) = unsafe {
            let status = inb(STATUS_PORT);
            if status & 0x01 == 0 {
                return None;
            }
            (status, inb(DATA_PORT))
        };
        if status & 0x20 != 0 {
            return None; // マウスのデータ
        }
        self.decode(scancode)
    }

    fn decode(&mut self, sc: u8) -> Option<u8> {
        if sc == 0xE0 {
            self.extended = true;
            return None;
        }
        let release = sc & 0x80 != 0;
        let code = sc & 0x7F;
        let ext = self.extended;
        self.extended = false;

        match code {
            0x2A | 0x36 if !ext => {
                self.shift = !release;
                return None;
            }
            0x1D => {
                self.ctrl = !release;
                return None;
            }
            0x3A if !ext => {
                if !release {
                    self.caps = !self.caps;
                }
                return None;
            }
            _ => {}
        }
        if release {
            return None;
        }
        if ext {
            // テンキーの Enter だけ対応。矢印キーなどは無視
            return if code == 0x1C { Some(b'\n') } else { None };
        }

        let mut ch = map_scancode(code, self.shift);
        if ch == 0 {
            return None;
        }
        if self.caps && ch.is_ascii_alphabetic() {
            ch = if self.shift {
                ch.to_ascii_lowercase()
            } else {
                ch.to_ascii_uppercase()
            };
        }
        if self.ctrl && ch.is_ascii_alphabetic() {
            return Some(ch.to_ascii_lowercase() & 0x1F); // Ctrl-C = 3
        }
        Some(ch)
    }
}
