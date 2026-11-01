#!/usr/bin/env bash
# QEMU で SimpleOS を起動する。
#   ./scripts/run_qemu.sh            ... ウィンドウ(VGA) + このターミナルのシリアルも有効
#   ./scripts/run_qemu.sh nographic  ... ウィンドウなし。ターミナルだけで操作(終了は Ctrl-A X)
#   ./scripts/run_qemu.sh direct     ... ISO を使わず kernel.bin を QEMU に直接渡す
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MODE="${1:-window}"
QEMU=qemu-system-x86_64

case "${MODE}" in
  window)
    exec "${QEMU}" -m 128M -cdrom simpleos.iso -serial stdio
    ;;
  nographic)
    exec "${QEMU}" -m 128M -cdrom simpleos.iso -nographic
    ;;
  direct)
    exec "${QEMU}" -m 128M -kernel kernel.bin -serial stdio
    ;;
  *)
    echo "usage: $0 [window|nographic|direct]" >&2
    exit 1
    ;;
esac
