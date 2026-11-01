ENTRY(_start)

SECTIONS
{
  . = 1M;

  /* Multiboot ヘッダはファイル先頭 8KiB 以内に置く必要がある */
  .text : ALIGN(16) {
    KEEP(*(.multiboot))
    *(.boot.text)
    *(.text .text.*)
  }

  .rodata : ALIGN(16) {
    *(.boot.rodata)
    *(.rodata .rodata.*)
  }

  .data : ALIGN(16) { *(.data .data.*) }

  .bss : ALIGN(4096) {
    *(.bss.boot)
    *(.bss .bss.*)
    *(COMMON)
  }

  /DISCARD/ : { *(.comment) *(.eh_frame) *(.eh_frame_hdr) *(.note.gnu.build-id) }
}
