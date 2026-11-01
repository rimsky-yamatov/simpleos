# SimpleOS boot code (Intel syntax)
# GRUB (Multiboot 1) enters _start in 32-bit protected mode.
# Enable paging, switch to 64-bit long mode, then call kmain.

.section .multiboot, "a"
.align 4
.long 0x1BADB002                    # magic
.long 0x00000003                    # flags: page-align modules + memory info
.long -(0x1BADB002 + 0x00000003)    # checksum

.section .boot.text, "ax"
.code32
.global _start
_start:
    cli
    cld
    mov esp, offset boot_stack_top
    mov dword ptr [mb_magic], eax
    mov dword ptr [mb_info], ebx

    # Identity-map the first 1GiB with 2MiB pages
    mov eax, offset p3_table
    or eax, 0x3
    mov dword ptr [p4_table], eax

    mov eax, offset p2_table
    or eax, 0x3
    mov dword ptr [p3_table], eax

    xor ecx, ecx
1:
    mov eax, 0x200000
    mul ecx
    or eax, 0x83                    # present | writable | huge(2MiB)
    mov dword ptr [p2_table + ecx*8], eax
    inc ecx
    cmp ecx, 512
    jne 1b

    mov eax, offset p4_table
    mov cr3, eax

    mov eax, cr4
    or eax, 0x20                    # CR4.PAE
    mov cr4, eax

    mov ecx, 0xC0000080             # IA32_EFER
    rdmsr
    or eax, 0x100                   # EFER.LME
    wrmsr

    mov eax, cr0
    or eax, 0x80000000              # CR0.PG
    mov cr0, eax

    lgdt [gdt64_ptr]
    # ljmp 0x08:long_mode_entry (opcode EA = far jmp ptr16:32 written by hand)
    .byte 0xEA
    .long long_mode_entry
    .word 0x08

.code64
long_mode_entry:
    xor eax, eax
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov esp, offset boot_stack_top
    mov edi, dword ptr [mb_info]
    mov esi, dword ptr [mb_magic]
    call kmain
2:
    cli
    hlt
    jmp 2b

.section .boot.rodata, "a"
.align 16
gdt64:
    .quad 0x0000000000000000        # null
    .quad 0x00209A0000000000        # 64bit code (L=1, P=1, DPL=0, exec/read)
    .quad 0x0000920000000000        # data
gdt64_end:
gdt64_ptr:
    .word gdt64_end - gdt64 - 1
    .long gdt64

.section .bss.boot, "aw", @nobits
.align 4096
p4_table:
    .skip 4096
p3_table:
    .skip 4096
p2_table:
    .skip 4096
.align 16
boot_stack:
    .skip 65536
boot_stack_top:
mb_info:
    .skip 4
mb_magic:
    .skip 4
