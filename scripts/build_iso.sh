#!/usr/bin/env bash
# SimpleOS の起動可能 ISO (simpleos.iso) を作る。
# 必要: rustup target add x86_64-unknown-none / grub-mkrescue (grub-pc-bin) / xorriso / mtools / binutils
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TARGET=x86_64-unknown-none
KERNEL_ELF64="target/${TARGET}/release/kernel"

# kernel クレートだけをベアメタル向けにビルドする(リンカスクリプトは kernel/build.rs が指定)
cargo build -p kernel --bin kernel --target "${TARGET}" --release

if [ ! -f "${KERNEL_ELF64}" ]; then
  echo "kernel binary not found: ${KERNEL_ELF64}" >&2
  exit 1
fi

# GRUB の Multiboot(v1) ローダは ELF32 しか読めないので、64bit ELF を ELF32 形式に変換する。
# (中身の機械語は 64bit のまま。boot.s が 32bit -> 64bit の切り替えを行う)
if command -v objcopy >/dev/null 2>&1; then
  OBJCOPY=objcopy
elif command -v llvm-objcopy >/dev/null 2>&1; then
  OBJCOPY=llvm-objcopy
else
  echo "objcopy (binutils) is required" >&2
  exit 1
fi
"${OBJCOPY}" -O elf32-i386 "${KERNEL_ELF64}" kernel.bin

# Multiboot ヘッダの検証(grub-file があれば)
if command -v grub-file >/dev/null 2>&1; then
  grub-file --is-x86-multiboot kernel.bin || {
    echo "kernel.bin is not a valid Multiboot image" >&2
    exit 1
  }
fi

rm -rf iso
mkdir -p iso/boot/grub
cp kernel.bin iso/boot/kernel.bin
cp kernel/grub.cfg iso/boot/grub/grub.cfg

grub-mkrescue -o simpleos.iso iso

ls -lh simpleos.iso kernel.bin
echo "OK: run ./scripts/run_qemu.sh"
