# kernel

SimpleOS のカーネル本体(`x86_64-unknown-none`, `no_std`)。

## 構成

| ファイル | 役割 |
| --- | --- |
| `src/boot.s` | Multiboot ヘッダ、32bit→64bit ロングモード移行、`kmain` 呼び出し |
| `src/main.rs` | `kmain`(ヒープ初期化 → BASIC シェル起動)、panic ハンドラ |
| `src/allocator.rs` | フリーリスト方式のヒープ(8MiB, 解放・結合に対応) |
| `src/vga.rs` | VGA テキスト出力(スクロール・カーソル) |
| `src/keyboard.rs` | PS/2 キーボード(US 配列, Shift / CapsLock / Ctrl) |
| `src/serial.rs` | COM1 シリアル入出力(`-nographic` で使える) |
| `src/console.rs` | 上記をまとめた `basic_core::Console` 実装 |
| `src/ramfs.rs` | メモリ上のファイルシステム(サンプル `.bas` を内蔵) |
| `src/mem.rs` | `memcpy` などの mem* 関数 |
| `linker.ld` / `build.rs` | 1MiB に配置するリンカスクリプトと、そのリンク指定 |
| `programs/*.bas` | 起動時に RAM ディスクへ入るサンプル |

BASIC 本体とシェル(`LIST` / `RUN` / `SAVE` など)は `basic_core` クレートにあり、
ホスト用の `emulator` と同じコードが動きます。

## ビルドと起動

```bash
rustup target add x86_64-unknown-none
./scripts/build_iso.sh          # simpleos.iso / kernel.bin を作る
./scripts/run_qemu.sh           # QEMU で起動
```
