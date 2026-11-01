# SimpleOS

Rust で書いた、古典的な BASIC を標準搭載した小さな OS です。
GRUB から起動し、画面(VGA)・キーボード・シリアルで BASIC シェルを操作できます。

- `basic_core` : BASIC インタプリタとシェル(`no_std`)。ホストでもカーネルでも同じコードが動く
- `emulator`   : ホスト上で動く BASIC シェル(動作確認・テスト用)
- `kernel`     : ブート可能な x86_64 カーネル

## OS として起動する

必要なもの: Rust (rustup)、`grub-mkrescue`(grub-pc-bin)、`xorriso`、`mtools`、binutils、QEMU

```bash
rustup target add x86_64-unknown-none
./scripts/build_iso.sh            # simpleos.iso と kernel.bin を作る
./scripts/run_qemu.sh             # ウィンドウ + シリアルで起動
./scripts/run_qemu.sh nographic   # ターミナルだけで操作 (終了: Ctrl-A, X)
```

起動すると `>` プロンプトが出ます。

```
> RUN DEMO
> DIR
> 10 PRINT "HELLO"
> 20 GOTO 10
> RUN            (Ctrl-C か Esc で中断)
> SAVE hello
> EXIT           (電源断)
```

ファイルは RAM 上にあるので、再起動すると消えます(サンプルは毎回自動で入ります)。

## ホストで試す (emulator)

```bash
cargo run --release -p simpleos-emulator              # 対話シェル
cargo run --release -p simpleos-emulator -- emulator/examples/primes.bas
cargo test -p basic_core                              # テスト
```

## BASIC の仕様

- 値は 64bit 整数のみ。変数名は英数字と `_`(大文字小文字は区別しない)
- 文: `LET` `PRINT` `INPUT` `GOTO` `GOSUB` `RETURN` `IF..THEN` `FOR..TO..STEP` `NEXT` `REM` `CLS` `END`
- `PRINT "文字列"; 式, 式` と書け、末尾に `;` を付けると改行しない
- 演算子: `+ - * / %` と括弧。条件: `= <> < > <= >=` と `AND` / `OR`
- `IF 条件 THEN 文` / `IF 条件 THEN 行番号`
- コマンド: `LIST` `RUN [名前]` `NEW` `SAVE` `LOAD` `DIR` `TYPE` `DEL` `VARS` `CLS` `VER` `HELP` `EXIT`
- 行番号だけを入力するとその行を削除

詳しい構成は `kernel/README.md`、今後の予定は `docs/roadmap.md` を参照してください。
