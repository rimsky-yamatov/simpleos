//! SimpleOS カーネル本体。
//!
//! GRUB(Multiboot 1) -> boot.s (32bit -> 64bit ロングモード) -> kmain -> BASIC シェル

#![no_std]
#![no_main]

extern crate alloc;

use core::arch::global_asm;
use core::fmt::Write;
use core::panic::PanicInfo;

use basic_core::{Shell, ShellAction};

mod allocator;
mod console;
mod keyboard;
mod mem;
mod port;
mod ramfs;
mod serial;
mod vga;

global_asm!(include_str!("boot.s"));

#[global_allocator]
static ALLOCATOR: allocator::FreeListAllocator = allocator::FreeListAllocator::new();

const HEAP_SIZE: usize = 8 * 1024 * 1024;

#[repr(align(16))]
struct HeapSpace([u8; HEAP_SIZE]);

static mut HEAP: HeapSpace = HeapSpace([0; HEAP_SIZE]);

const MULTIBOOT_MAGIC: u32 = 0x2BAD_B002;

#[no_mangle]
pub extern "C" fn kmain(_mb_info: u32, mb_magic: u32) -> ! {
    unsafe {
        ALLOCATOR.init(core::ptr::addr_of_mut!(HEAP) as usize, HEAP_SIZE);
    }

    let mut console = console::KernelConsole::new();
    let mut storage = ramfs::RamFs::with_samples();
    let mut shell = Shell::new();

    console.print("SimpleOS booted (x86_64, long mode)\n");
    if mb_magic != MULTIBOOT_MAGIC {
        console.print("warning: not started by a Multiboot loader\n");
    }
    shell.banner(&mut console);
    console.print("Try:  RUN DEMO   /   DIR   /   LIST\n");

    loop {
        console.print("\n> ");
        let line = console.input_line();
        if shell.handle_line(&line, &mut console, &mut storage) == ShellAction::Exit {
            break;
        }
    }

    console.print("\nSystem halted. It is now safe to power off.\n");
    port::poweroff()
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut screen = vga::Vga::new();
    screen.set_color(0x4F); // 白文字・赤背景
    screen.clear();
    let _ = writeln!(screen, "KERNEL PANIC: {}", info);
    let _ = writeln!(serial::SerialWriter, "\nKERNEL PANIC: {}", info);
    port::halt_forever()
}
