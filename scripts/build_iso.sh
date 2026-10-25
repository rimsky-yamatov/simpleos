#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

mkdir -p iso/boot/grub

cargo build --target x86_64-unknown-none --verbose

if [ ! -f "target/x86_64-unknown-none/debug/kernel" ]; then
  echo "kernel binary not found: target/x86_64-unknown-none/debug/kernel"
  exit 1
fi

ld -n --gc-sections -T kernel/linker.ld -o kernel.bin target/x86_64-unknown-none/debug/kernel
cp kernel.bin iso/boot/kernel.bin
cp kernel/grub.cfg iso/boot/grub/grub.cfg

grub-mkrescue -o simpleos.iso iso

ls -lh simpleos.iso
