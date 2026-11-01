//! x86 I/O ポートとシステム停止まわりの低レベル操作。

use core::arch::asm;

#[inline]
pub unsafe fn outb(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
}

#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
    value
}

#[inline]
pub unsafe fn outw(port: u16, value: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") value, options(nomem, nostack, preserves_flags));
}

/// 割り込みを止めて永久に停止する。
pub fn halt_forever() -> ! {
    loop {
        unsafe {
            asm!("cli", "hlt", options(nomem, nostack));
        }
    }
}

/// 電源断を試みる(QEMU / Bochs / VirtualBox)。失敗したら停止する。
pub fn poweroff() -> ! {
    unsafe {
        outw(0x604, 0x2000); // QEMU (q35 / 新しめの i440fx)
        outw(0xB004, 0x2000); // Bochs / 古い QEMU
        outw(0x4004, 0x3400); // VirtualBox
    }
    halt_forever()
}
