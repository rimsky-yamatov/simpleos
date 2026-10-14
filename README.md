# SimpleOS

Simple OS project — Rustで書く、古典的なBASICを標準搭載した小さなOSの実験リポジトリ。

このリポジトリには2つの主要な部分があります:

- emulator: ホスト環境（通常のRustバイナリ）として動作する簡易BASICインタプリタ。まずはこちらが動くことを確認できます。
- kernel: 将来的にブート可能なカーネルを作るためのプレースホルダとメモ。ブートイメージ化・qemuでの起動は今後の作業です。

まずは emulator をビルドして起動し、BASICのREPLを試してください。

使い方（emulator）:

```bash
# 必要: Rust toolchain
cd emulator
cargo run --release
```

REPLのコマンド（簡易）:
- LET <変数> = <式>
- PRINT <式>
- EXIT または QUIT で終了
- 式だけを入力すると評価して表示します。

例:
```
LET A = 10
PRINT A + 5
```

今後の予定:
- kernel を bootimage / bootloader ベースでブート可能にする
- BASIC をカーネル内に移植して、実機（qemu）上で動作するOSにする

