# Naire

> Flexible Renamer 後継の一括リネームツール（Tauri v2 + React + TypeScript）

## プロジェクト概要

Windows / macOS 対応の一括リネームツール。正規表現・ワイルドカード・定型・簡易マクロに対応し、日本語ファイル名操作（全半角・仮名変換・濁音/半濁音除去）を完全サポートする。詳細仕様は `HANDOFF_naire.md` を参照。

## 絶対に実装しないこと

- タグリネーム（ID3 / EXIF）
- ファイル属性・タイムスタンプ変更
- 連番オブジェクト生成
- REDO
- ネットワークドライブのブラウズ
- スクリプト機能（FR の VBScript/JScript 拡張、Excel COM 連携等）
- フォルダ振り分け（リネーム時に別フォルダへ移動）
- `*` ランダム数字変数

## 命名規則

- Rust: snake_case
- TypeScript: camelCase（型は PascalCase）
- コンポーネント: PascalCase

## アーキテクチャ原則

- プレビュー計算は必ず Rust 側（`commands.rs` → `preview_rename`）で行う。フロントエンドで文字列処理を行わない
- 文字変換は `src-tauri/src/rename/convert.rs` に集約する
- 表示フィルタは `src-tauri/src/filter.rs` に集約。`globset` を使用しセミコロン区切り＋ `^(...)` 否定をサポート
- フィルタ適用は `list_entries` / `preview_rename` / `execute_rename` の Rust 側で行い、フロントには絞り込み済みリストのみ返す
- 連番フォーマッタは `src-tauri/src/rename/sequence.rs` に集約。10 進 / 16 進大文字 / 英大文字 Excel 列名スタイルの 3 種をサポート
- `start` / `step` は 0 以上 / 1 以上に制約（負の値・0 ステップは UI 側でも入力ガード）
- グローバル `SequenceConfig` は advanced 系ステップのみに適用。定型 op 内 `start` / `step` / `digits` は op 側を優先する
- 検索 / 置換 入力欄のサポートボタンは `src/components/SupportButton/` に集約。項目定義は `support-items.ts`、挿入ロジックは `insert-at-cursor.ts` に分離
- サポートメニュー項目はモード × フィールドの 6 コンテキストごとに定義（`wildcard.search` / `.replace`, `regex.search` / `.replace`, `char.search` / `.replace`）
- マクロ専用変数 `\orig` のサポート項目はマクロ編集時の advanced ステップでのみ表示する
- マクロ実行はステップ処理方式（1 クリック = 1 ステップ）。`apply_macro_step` を都度 invoke してプレビュー更新
- マクロの `[戻す]` は何ステップ進んでいてもマクロ開始前（step 0）に巻き戻す。途中ステップへの戻りは持たない
- マクロ進行中（`step_index > 0`）はマクロ選択ドロップダウンをロックする
- ファイルシステムへの実際のリネームは `apply_rename_to_filesystem` でのみ行う。プレビューはメモリ上の状態
- 部分選択時は対象を選択行に絞る。連番は選択行のみで進める。非選択行はプレビューに残す
- 濁音・半濁音除去は NFD → U+3099 / U+309A 除去 → NFC（`unicode-normalization` クレート）
- `\orig` 変数はマクロ専用。`MacroContext::original_name` に保持し `variables.rs` で展開する
- 日時系変数（`\Y \y \m \d \H \I \M \S \p \a \A \b \B`）は **ファイル mtime ベース**（`std::fs::Metadata::modified()` を `chrono::DateTime<Local>` に変換）。現在日時ではない
- ロケール依存変数（`\a \A \b \B \p`）は `chrono::format::Locale` で OS ロケールから決定
- UNDO スタック上限は 20 件（処理単位）。永続化しない
- マクロの fold 実行は `macro_runner.rs` の `run_macro` に集約する
- マクロ IO（インポート／エクスポート）は `macro_io.rs` に集約する
- インポート時は `id` を必ず再生成する（重複防止）
- エクスポートは単一マクロ＝オブジェクト、複数＝配列で書き分ける

## Task Management

- **task_file**: `docs/tasks.md`（未作成。実装フェーズ移行時に追加）
- **done_marker**: `[x]`
- **progress_summary**: true

## Documentation

- **docs_to_update**: 未定（README は実装初期段階で追加予定）
- **doc_pairs**: 未定

## Versioning

- **version_files**:
  - `package.json`
  - `src-tauri/Cargo.toml`
  - `src-tauri/tauri.conf.json`
- **extra_version_files**: none
- **cargo_lockfile**: true

## CI/CD

- **cicd**: true
- **cicd_trigger**: tag push（`v*.*.*` 形式）
- **cicd_platform**: GitHub Actions（Windows x86_64 + macOS universal）
- **cicd_note**: タグプッシュで自動ビルド & Release ドラフト作成。`ci.yml` は push/PR で Rust 単体テスト + フロントビルドのみ実行

## SNS

- **sns_accounts**: none
