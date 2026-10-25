#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[link_section = ".multiboot"]
#[used]
static MULTIBOOT_HEADER: [u32; 4] = [
    0x1BADB002,
    0x00000003,
    (-(0x1BADB002 + 0x00000003)) as u32,
    0,
];
