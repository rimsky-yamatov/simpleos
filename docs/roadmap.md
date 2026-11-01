# SimpleOS 実装ロードマップ

## 完了
- Rust ワークスペース(basic_core / emulator / kernel)
- BASIC: 行番号プログラム、LET / PRINT(文字列・`;` `,`)/ INPUT / GOTO / GOSUB / RETURN /
  IF..THEN(任意の文、AND / OR)/ FOR..TO..STEP / NEXT / REM / CLS / END
- 共有シェル(`basic_core::Shell`): LIST / RUN / NEW / SAVE / LOAD / DIR / TYPE / DEL / VARS
- 起動可能なカーネル: GRUB(Multiboot1) → 32bit→64bit 移行 → VGA / PS/2 / シリアル → BASIC シェル
- ヒープアロケータ、RAM ファイルシステム、Ctrl-C / Esc による実行中断
- GRUB で ISO 化し、QEMU で起動(CI でも ISO をビルド)

## 次の候補
- IDT を設置して CPU 例外(0除算・ページフォールトなど)を panic として表示する
- PIT / キーボード割り込み化(`hlt` で CPU を休ませる)
- ATA(PIO)ディスクドライバによる SAVE / LOAD の永続化
- 文字列変数・配列・RND などの BASIC 拡張
- ディレクトリ構造を持つ仮想ファイルシステム
