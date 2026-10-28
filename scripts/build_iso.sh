#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TARGET=x86_64-unknown-none
KERNEL_ELF="target/${TARGET}/debug/kernel"

mkdir -p iso/boot/grub

# kernel クレートだけをベアメタル向けにビルドする
# (ワークスペース全体だと std を使う emulator まで対象になり失敗する)
RUSTFLAGS="-C link-arg=-T${ROOT_DIR}/kernel/linker.ld -C link-arg=-n -C link-arg=--gc-sections" \
  cargo build -p kernel --bin kernel --target "${TARGET}" --verbose

if [ ! -f "${KERNEL_ELF}" ]; then
  echo "kernel binary not found: ${KERNEL_ELF}"
  exit 1
fi

# すでにリンカスクリプトでリンク済みなので、そのままコピーする
cp "${KERNEL_ELF}" kernel.bin
cp kernel.bin iso/boot/kernel.bin
cp kernel/grub.cfg iso/boot/grub/grub.cfg

grub-mkrescue -o simpleos.iso iso

ls -lh simpleos.iso
