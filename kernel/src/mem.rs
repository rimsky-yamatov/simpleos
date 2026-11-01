//! `core` / `alloc` が要求する mem* 関数。
//! ベアメタルでは libc が無いので自前で用意する。
//! (コンパイラによって自分自身への呼び出しに変換されないよう、
//!  rep 命令かインラインの volatile アクセスで実装している)

use core::arch::asm;
use core::ptr::{read_volatile, write_volatile};

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    asm!(
        "rep movsb",
        inout("rcx") n => _,
        inout("rdi") dest => _,
        inout("rsi") src => _,
        options(nostack, preserves_flags)
    );
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8 {
    asm!(
        "rep stosb",
        inout("rcx") n => _,
        inout("rdi") dest => _,
        in("al") c as u8,
        options(nostack, preserves_flags)
    );
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if (dest as usize) <= (src as usize) || (dest as usize) >= (src as usize).wrapping_add(n) {
        // 前方コピーで安全
        memcpy(dest, src, n);
    } else {
        // 領域が重なっていて dest が後ろにあるので、後ろからコピーする
        let mut i = n;
        while i > 0 {
            i -= 1;
            write_volatile(dest.add(i), read_volatile(src.add(i)));
        }
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let x = read_volatile(a.add(i));
        let y = read_volatile(b.add(i));
        if x != y {
            return x as i32 - y as i32;
        }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn bcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    memcmp(a, b, n)
}
