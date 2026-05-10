# HANDOFF — Naire

ファイル・フォルダ一括リネームツール。Flexible Renamer（最終更新 2011 年）の後継として自作。

- **リポジトリ名**: `naire`
- **productName**: `Naire`
- **bundle identifier**: `com.polarissolutions.naire`（`tauri.conf.json` で設定）

---

## 概要

Windows / macOS 対応の一括リネームツール。
正規表現・ワイルドカード・定型・簡易マクロに対応し、
日本語ファイル名操作（全半角・仮名変換・濁音除去）を完全サポートする。

**スコープ内**
- 正規表現・ワイルドカード・定型・簡易マクロによるリネーム
- 日本語ファイル名操作（全半角・仮名変換・濁音/半濁音除去）の完全サポート
- サブフォルダ再帰処理・リアルタイムプレビュー・複数段 UNDO（最大 20 処理）
- マクロの JSON インポート／エクスポート（Claude 連携用途を含む）
- 表示フィルタ（グロブパターンで一覧表示を絞り込み・履歴とプリセット内蔵）

**スコープ外（実装しない）**
- タグリネーム（ID3 / EXIF）
- ファイル属性・タイムスタンプ変更
- 連番オブジェクト生成
- REDO
- ネットワークドライブのブラウズ
- スクリプト機能（FR の VBScript/JScript 拡張、Excel COM 連携等） — Windows 限定技術かつ非推奨。マクロ機能で代替可能
- フォルダ振り分け（リネーム時に別フォルダへ移動）— Naire はリネームのみ
- `*` ランダム数字変数 — 再現性なし

---

## 技術スタック

| 層 | 技術 |
|---|---|
| フレームワーク | Tauri v2 |
| フロントエンド | React 19 + TypeScript + Vite |
| スタイリング | CSS Modules |
| バックエンド | Rust（stable） |
| 正規表現 | `regex` クレート |
| サブフォルダ再帰 | `walkdir` クレート |
| 日付処理 | `chrono` クレート |
| Unicode 正規化 | `unicode-normalization` クレート |
| 設定永続化 | `tauri-plugin-store` |
| ファイルダイアログ | `tauri-plugin-dialog` |
| シリアライズ | `serde` + `serde_json` |
| D&D（マクロ） | `dnd-kit`（フロントエンド） |

---

## UI レイアウト

```
┌─────────────────────────────────────────────────────────────────┐
│  [● ファイル] [○ フォルダ]  フィルタ:[*▼]  ☑ サブフォルダ再帰 深さ[0] [歯車] │  ← ツールバー
├────────────┬──────────────────────┬──────────────────────────────┤
│            │ [定型][高度な][マクロ]                        │
│  フォルダ   ├──────────────────────┼──────────────────────────────┤
│  ツリー    │                      │  現在の名前       新しい名前  │
│  （前回の  │  設定パネル          │  ─────────────────────────── │
│  パスを    │  （モード別 UI）      │  file_a.jpg  →  img_001.jpg  │
│  記憶）    │                      │  file_b.jpg  →  img_002.jpg  │
│            ├──────────────────────┤  ...                         │
│            │ 連番: ☑フォルダごとリセット [▶進数] │                │
│            │ 開始[0] ステップ[1]   │                              │
├────────────┴──────────────────────┴──────────────────────────────┤
│   [リネーム実行]     [元に戻す Ctrl+Z]     [クリア]              │
└─────────────────────────────────────────────────────────────────┘
```

連番設定パネルは中央ペイン下部に**常時表示**（`定型` / `高度な` / `マクロ` のどのタブでも見える）。advanced 全モードで使われる `?` `??` `???` `????` 変数のカウンタ動作を制御する。

---

## 型定義（TypeScript）

### `src/types/rename.ts`

```typescript
// ── ターゲット ──────────────────────────────────────────────────
export type TargetType = "file" | "folder";

// ── リネームステップ（Union）───────────────────────────────────
export type RenameStep =
  | { kind: "builtin";       op: BuiltinOp }
  | { kind: "wildcard";     search: string; replace: string }
  | { kind: "regex";        search: string; replace: string }
  | { kind: "char_convert"; from: string;   to: string };

// ── 定型操作 ────────────────────────────────────────────
export type BuiltinOp =
  // 連番・文字列の追加
  | { type: "add_seq_str";         prefix: string; suffix: string; digits: number; start: number; step: number }
  | { type: "add_datetime";        format: string; position: "prefix" | "suffix" }
  | { type: "add_folder_name";     position: "prefix" | "suffix" }
  | { type: "add_folder_seq";      digits: number; start: number; step: number }
  | { type: "truncate_from_start"; n: number }
  | { type: "truncate_from_end";   n: number }
  // 数字・文字列の削除
  | { type: "delete_chars";    from: "start" | "end"; offset: number; count: number }
  | { type: "delete_before";   from: "start" | "end"; n: number }
  | { type: "delete_pattern";  pattern: DeletePattern }
  | { type: "make_83" }
  // 文字種の変換
  | { type: "case_convert";       conversion: CaseConversion;  skip_ext: boolean }
  | { type: "kana_convert";       conversion: KanaConversion;  skip_ext: boolean }
  | { type: "width_convert";      conversion: WidthConversion; skip_ext: boolean }
  | { type: "clear_diacritics";   skip_ext: boolean }
  | { type: "remove_voiced_mark"; skip_ext: boolean }   // 濁音・半濁音除去（NEW）
  // 文字列の置換
  | { type: "string_replace"; search: string; replace: string }
  // 数値の整理
  | { type: "number_pad";    from: "start" | "end"; n: number; digits: number }
  | { type: "number_adjust"; from: "start" | "end"; n: number; delta: number }
  // 拡張子の変換
  | { type: "ext_convert";  conversion: ExtConversion }
  | { type: "ext_delete" }
  | { type: "ext_add";      ext: string }
  | { type: "ext_replace";  ext: string };

export type DeletePattern =
  | "copy_num"          // "コピー (数字)"〜"
  | "copy_num_vista"    // "ーコピー (数字)" (Vista)
  | "shortcut"          // "^へのショートカット"
  | "shortcut_vista"    // "ーショートカット" (Vista)
  | "bracket_content"   // 括弧とその中身（全種）
  | "num_kagi_or_round" // 【数字】または（数字）
  | "num_kagi"          // 【数字】
  | "num_round"         // （数字）
  | "num_lenticular";   // 〔数字〕

export type CaseConversion  = "capitalize" | "upper" | "lower";
export type KanaConversion  = "hira_to_kata" | "kata_to_hira"
                            | "hankaku_kata_to_zenkaku" | "zenkaku_kata_to_hankaku";
export type WidthConversion = "to_zenkaku" | "to_hankaku";
export type ExtConversion   = "upper" | "lower";

// ── マクロ ───────────────────────────────────────────────────
export interface Macro {
  id:         string;
  name:       string;
  steps:      RenameStep[];
  created_at: string;  // ISO 8601
  updated_at: string;
}

// ── UNDO ─────────────────────────────────────────────────────
// 1件 = 1回の execute_rename 呼び出し。複数ファイルをまとめて逆適用する。
export interface RenameOp {
  old_path: string;
  new_path: string;
}
export interface RenameRecord {
  id:        string;
  timestamp: string;
  ops:       RenameOp[];  // そのバッチで変更した全ファイルのペア
}
// スタック上限: 20件（処理単位）。1件の中に何千ファイルが含まれていても1件扱い。

// ── プレビュー行 ─────────────────────────────────────────────
export interface PreviewItem {
  original:   string;
  renamed:    string;
  is_changed: boolean;
}

// ── 連番カウンタ設定（グローバル）────────────────────────────
// advanced 全モード（regex/wildcard/char_convert）の ? ?? ??? ???? 変数に適用される。
// 定型ステップが個別に持つ start/step は、ここで設定したグローバル値より優先される。
export interface SequenceConfig {
  start:            number;                       // 開始値（0 以上）
  step:             number;                       // 増分（1 以上）
  reset_per_folder: boolean;                      // recursive=true 時、各フォルダ進入時にカウンタを start に戻す
  numbering:        "decimal" | "hex" | "alpha";  // 進数（10進 / 16進大文字 / 英大文字 Excel 列名スタイル）
}

// ── アプリ設定 ───────────────────────────────────────────────
export interface AppConfig {
  last_folder:    string;
  last_mode:      "builtin" | "advanced" | "macro";
  last_target:    TargetType;
  recursive:      boolean;    // サブフォルダ再帰の ON/OFF
  depth:          number;     // recursive=true 時のみ有効。0=無制限、N=N 階層まで
  filter:         string;     // 表示フィルタ。既定 "*"
  filter_history: string[];   // 直近フィルタ（最大 10 件）
  seq:            SequenceConfig;  // 連番カウンタのグローバル設定
  window:         { width: number; height: number; x?: number; y?: number };
  macros:         Macro[];
}
```

---

## コンポーネント構成

```
src/
├── App.tsx
├── components/
│   ├── Toolbar.tsx               # ターゲット切り替え・再帰チェックボックス・深さ入力・表示フィルタ入力
│   ├── FolderTree.tsx            # 左ペイン：フォルダツリー（前回パス記憶）
│   ├── ModePanel/
│   │   ├── ModePanel.tsx         # 中央ペイン：タブ切り替え
│   │   ├── builtin/
│   │   │   ├── BuiltinMenu.tsx    # カテゴリ折りたたみツリー
│   │   │   └── BuiltinParams.tsx  # 選択定型のパラメータフォーム（インライン）
│   │   ├── advanced/
│   │   │   ├── WildcardMode.tsx
│   │   │   ├── RegexMode.tsx
│   │   │   └── CharConvertMode.tsx
│   │   └── macro/
│   │       ├── MacroEditor.tsx   # ステップリスト（dnd-kit で D&D 並べ替え）
│   │       └── MacroList.tsx     # 保存済みマクロ一覧
│   ├── SequencePanel.tsx         # 中央ペイン下部：連番カウンタ設定（常時表示）
│   ├── SupportButton/
│   │   ├── SupportButton.tsx     # 検索/置換フィールド横の「サポート▶」ボタン + Popover
│   │   ├── support-items.ts      # モード×フィールド別の項目定義
│   │   └── insert-at-cursor.ts   # 入力欄のカーソル位置にテキスト挿入
│   ├── PreviewPanel.tsx          # 右ペイン：変更前後テーブル
│   └── ActionBar.tsx             # リネーム実行・UNDO・クリア
├── hooks/
│   ├── useRename.ts              # invoke("execute_rename") + UNDO スタック管理
│   ├── usePreview.ts             # invoke("preview_rename") → PreviewItem[]
│   └── useConfig.ts              # tauri-plugin-store で AppConfig 読み書き
└── types/
    └── rename.ts
```

---

## Tauri コマンド一覧

```rust
// src-tauri/src/commands.rs

/// リネーム結果のプレビュー生成（ファイルは変更しない）
#[tauri::command]
pub async fn preview_rename(
    folder:    String,
    steps:     Vec<RenameStepDto>,
    target:    TargetType,
    recursive: bool,
    depth:     u32,                // recursive=true 時のみ有効。0=無制限、N=N 階層
    filter:    String,             // 表示フィルタ（"*" で全件）
    seq:       SequenceConfigDto,  // 連番カウンタ（advanced 全モードで使用）
) -> Result<Vec<PreviewItem>, String>

/// リネーム実行（UNDO 用レコードを返す）
#[tauri::command]
pub async fn execute_rename(
    folder:    String,
    steps:     Vec<RenameStepDto>,
    target:    TargetType,
    recursive: bool,
    depth:     u32,
    filter:    String,
    seq:       SequenceConfigDto,
) -> Result<RenameRecord, String>

/// UNDO（RenameRecord の逆方向リネームをまとめて実行）
#[tauri::command]
pub async fn undo_rename(record: RenameRecord) -> Result<(), String>

/// フォルダ内エントリ一覧取得（filter で絞り込み済みのリストを返す）
#[tauri::command]
pub async fn list_entries(
    folder:    String,
    target:    TargetType,
    recursive: bool,
    depth:     u32,
    filter:    String,
) -> Result<Vec<String>, String>

/// マクロを JSON ファイルとしてエクスポート（保存ダイアログを開く）
/// 単一マクロ: Macro オブジェクト、複数: Macro[] を書き出す
#[tauri::command]
pub async fn export_macros(
    app:    tauri::AppHandle,
    macros: Vec<Macro>,        // 1件でも Vec で統一
) -> Result<(), String>

/// JSON ファイルからマクロをインポート（開くダイアログを開く）
/// 単一オブジェクト・配列どちらも受け付け、検証後に返す
#[tauri::command]
pub async fn import_macros(
    app: tauri::AppHandle,
) -> Result<Vec<Macro>, String>
```

---

## Rust モジュール構成

```
src-tauri/src/
├── main.rs
├── commands.rs           # Tauri コマンド定義
├── config.rs             # AppConfig 読み書き（tauri-plugin-store）
├── filter.rs             # 表示フィルタ（globset でセミコロン区切りパターン解析）
├── macro_io.rs           # マクロ JSON インポート／エクスポート
└── rename/
    ├── mod.rs            # エントリポイント・ステップ dispatch
    ├── builtin.rs         # 定型処理（全 46 種）
    ├── advanced.rs       # ワイルドカード・正規表現処理
    ├── convert.rs        # 文字変換（仮名・全半角・濁音除去）
    ├── macro_runner.rs   # マクロ：ステップを fold で順次適用
    ├── sequence.rs       # 連番フォーマッタ（10進/16進大文字/英大文字 Excel 列名）
    ├── variables.rs      # 置換変数展開（\f \e \t \Y\m\d \orig 等。日時はファイル mtime ベース）
    └── undo.rs           # RenameRecord の逆適用
```

---

## 文字変換実装（Rust）

### `src-tauri/src/rename/convert.rs`

```rust
use unicode_normalization::UnicodeNormalization;

/// ひらがな → カタカナ（U+3041–U+3096 → U+30A1–U+30F6）
pub fn hiragana_to_katakana(s: &str) -> String {
    s.chars().map(|c| {
        if ('\u{3041}'..='\u{3096}').contains(&c) {
            char::from_u32(c as u32 + 0x60).unwrap_or(c)
        } else { c }
    }).collect()
}

/// カタカナ → ひらがな（U+30A1–U+30F6 → U+3041–U+3096）
pub fn katakana_to_hiragana(s: &str) -> String {
    s.chars().map(|c| {
        if ('\u{30A1}'..='\u{30F6}').contains(&c) {
            char::from_u32(c as u32 - 0x60).unwrap_or(c)
        } else { c }
    }).collect()
}

/// 全角英数記号 → 半角（U+FF01–U+FF5E → U+0021–U+007E）
pub fn zenkaku_to_hankaku_alnum(s: &str) -> String {
    s.chars().map(|c| {
        if ('\u{FF01}'..='\u{FF5E}').contains(&c) {
            char::from_u32(c as u32 - 0xFEE0).unwrap_or(c)
        } else { c }
    }).collect()
}

/// 濁音・半濁音除去（ひらがな・カタカナ共通）
///
/// 実装方針: NFD 正規化により合成済み仮名を「基底文字 + 結合文字」に分解し、
/// 結合濁点 U+3099 と 結合半濁点 U+309A を除去したあと NFC 再合成する。
///
/// 例: が(U+304C) --NFD--> か(U+304B) + ゛(U+3099) --strip--> か --NFC--> か
///     ぱ(U+3071) --NFD--> は(U+306F) + ゜(U+309A) --strip--> は --NFC--> は
///     ゔ(U+3094) --NFD--> う(U+3046) + ゛(U+3099) --strip--> う --NFC--> う
///     ガ(U+30AC) --NFD--> カ(U+30AB) + ゛(U+3099) --strip--> カ --NFC--> カ
pub fn remove_voiced_marks(s: &str) -> String {
    s.nfd()
        .filter(|&c| c != '\u{3099}' && c != '\u{309A}')
        .collect::<String>()
        .nfc()
        .collect()
}

/// ダイアクリティカルマーク除去（ラテン文字向け）
/// NFD → Mn（Non-spacing Mark）カテゴリを除去 → NFC
pub fn clear_diacritics(s: &str) -> String {
    s.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect::<String>()
        .nfc()
        .collect()
}

/// 拡張子を除いた部分にのみ変換を適用するラッパー
pub fn apply_skip_ext<F: Fn(&str) -> String>(filename: &str, f: F) -> String {
    match filename.rfind('.') {
        Some(dot) => format!("{}{}", f(&filename[..dot]), &filename[dot..]),
        None      => f(filename),
    }
}
```

---

## 高度なリネーム — 置換変数仕様

> **重要**: 日時系変数（`\Y \y \m \d \H \I \M \S \p \a \A \b \B`）は **対象ファイルの mtime（更新日時）** を参照する。「現在日時」ではない。Flexible Renamer の挙動と一致させるため、`std::fs::Metadata::modified()` で取得した値を `chrono::DateTime<Local>` に変換して展開する。

### ファイル関連

| 変数 | 意味 | Rust 実装 |
|------|------|-----------|
| `\0` | 現在のファイル名（拡張子含む） | そのまま |
| `\t` | ファイルタイトル（拡張子除く） | `Path::file_stem()` |
| `\e` | 現在の拡張子（ドット含む） | `Path::extension()` に `.` を付加 |
| `\f` | 所属フォルダ名 | `path.parent()?.file_name()` |
| `\F` | 親フォルダ名 | `path.parent()?.parent()?.file_name()` |
| `\;` | ファイルサイズ（桁区切りあり） | `Metadata::len()` を 3 桁区切りで整形 |
| `\:` | ファイルサイズ（桁区切りなし） | `Metadata::len()` をそのまま 10 進文字列化 |

### 連番

| 変数 | 意味 | Rust 実装 |
|------|------|-----------|
| `?` | 1桁連番 | カウンタ |
| `??` | 2桁連番 | カウンタ（`{:02}`） |
| `???` | 3桁連番 | カウンタ（`{:03}`） |
| `????` | 4桁連番 | カウンタ（`{:04}`） |

### 日時（**ファイルの mtime ベース**）

| 変数 | 意味 | Rust 実装 |
|------|------|-----------|
| `\Y` | 4桁西暦年 | `chrono` `%Y` |
| `\y` | 2桁西暦年（00-99） | `chrono` `%y` |
| `\m` | 月（01-12, 2桁） | `chrono` `%m` ※正規表現の `\d`（数字）と別物 |
| `\d` | 日（01-31, 2桁） | `chrono` `%d` |
| `\H` | 時（00-23, 24時間表記） | `chrono` `%H` |
| `\I` | 時（00-12, 12時間表記） | `chrono` `%I` |
| `\M` | 分（00-59） | `chrono` `%M` |
| `\S` | 秒（00-59） | `chrono` `%S` |
| `\p` | 午前 / 午後 | `chrono` `%p`（ロケール依存） |
| `\a` | 曜日の省略名 | `chrono` `%a`（ロケール依存：JA → 月、EN → Mon） |
| `\A` | 曜日の正式名 | `chrono` `%A`（ロケール依存：JA → 月曜日、EN → Monday） |
| `\b` | 月の省略名 | `chrono` `%b` |
| `\B` | 月の正式名 | `chrono` `%B` |

**修飾子**

| 表記 | 意味 |
|------|------|
| `\#X` | `X` の先行ゼロを削除（例: `\#m` = `5`、`\#d` = `9`）|

> `\a \A \b \B \p` のロケールは `chrono::format::Locale` で OS 環境変数（`LC_TIME` / Windows のロケール）から決定する。

### 正規表現キャプチャ・大文字小文字制御

| 変数 | 意味 | Rust 実装 |
|------|------|-----------|
| `\1`〜`\9` | キャプチャグループ | regex の `${1}`〜`${9}` |
| `\u` | 次の 1 文字を大文字 | 後処理で展開 |
| `\U` | `\E` まで大文字 | 後処理で展開 |
| `\l` | 次の 1 文字を小文字 | 後処理で展開 |
| `\L` | `\E` まで小文字 | 後処理で展開 |
| `\E` | `\U` `\L` の効果終了 | 後処理で展開 |

### マクロ専用

| 変数 | 意味 | Rust 実装 |
|------|------|-----------|
| `\orig` | **マクロ開始時点の元ファイル名** | `MacroContext::original_name` |

> `\orig` はマクロのステップ内でのみ有効。非マクロモード（高度なリネーム単体）では展開されない。

### スコープ外（Flexible Renamer にあるが Naire では実装しない）

| 変数 | 意味 | 理由 |
|------|------|------|
| `*` | ランダム数字 | 再現性なし、用途が稀 |
| `..` `/` `\P` | フォルダ振り分け（パス操作） | Naire はリネームのみで移動・配置換えは行わない |
| 任意 strftime（`\j` 等） | `chrono::format::strftime` 互換 | 将来検討（`\#X` で日時用途は概ねカバー可能） |

**ワイルドカード → 正規表現 変換規則**

```
?  →  .      （任意の 1 文字）
*  →  .*     （任意の文字列）
.  →  \.     （ピリオドのエスケープ）
```

## 表示フィルタ仕様

ツールバーの「フィルタ」入力欄でファイル/フォルダ一覧の表示を絞り込む。リネームエンジンに渡される前段の絞り込みのため、フィルタを通過したエントリのみがプレビュー・実行対象になる。

### 構文

| 記法 | 意味 | 例 |
|------|------|-----|
| `*` | すべて表示（既定値） | `*` |
| `pattern` | グロブパターンに合致するもの | `*.jpg` |
| `pat1;pat2;pat3` | セミコロン区切りで OR | `*.jpg;*.png;*.gif` |
| `^(...)` | 否定（除外）| `^(*.tmp;*.bak)` |

`*` `?` のワイルドカード意味は他のモードと同じ。複数パターンと否定は組み合わせ不可（FR の仕様に合わせる）。

### 組み込みプリセット

ドロップダウンに以下のプリセットを表示する（ユーザーは編集可能、履歴と併存）。

| 名称 | パターン |
|------|---------|
| すべて | `*` |
| 画像 | `*.bmp;*.dib;*.gif;*.jpg;*.jpeg;*.jpe;*.png;*.ico;*.cur;*.webp;*.heic;*.heif` |
| 動画 | `*.mp4;*.mov;*.avi;*.mkv;*.wmv;*.flv;*.webm;*.m4v` |
| 音楽 | `*.mp3;*.wav;*.flac;*.aac;*.m4a;*.ogg;*.wma` |
| ドキュメント | `*.txt;*.doc;*.docx;*.xls;*.xlsx;*.ppt;*.pptx;*.pdf;*.md;*.rtf` |
| アーカイブ | `*.zip;*.rar;*.7z;*.tar;*.gz;*.bz2;*.xz` |
| 隠しファイル除外 | `^(.*)` |

### 履歴

`AppConfig.filter_history` に直近 10 件を保持。フィルタを変更してリネーム実行 or プレビュー確定したタイミングで先頭に追加（重複は削除して再追加）。

### Rust 実装方針

`globset` クレートを使用。

```rust
// src-tauri/src/filter.rs

use globset::{Glob, GlobSet, GlobSetBuilder};

pub struct DisplayFilter {
    matchers: GlobSet,
    negated:  bool,
}

impl DisplayFilter {
    pub fn parse(pattern: &str) -> Result<Self, String> {
        let trimmed = pattern.trim();
        let (pat, negated) = if let Some(inner) = trimmed.strip_prefix("^(").and_then(|s| s.strip_suffix(')')) {
            (inner, true)
        } else {
            (trimmed, false)
        };

        let mut builder = GlobSetBuilder::new();
        for token in pat.split(';').filter(|s| !s.trim().is_empty()) {
            let glob = Glob::new(token.trim()).map_err(|e| e.to_string())?;
            builder.add(glob);
        }
        Ok(Self { matchers: builder.build().map_err(|e| e.to_string())?, negated })
    }

    pub fn matches(&self, name: &str) -> bool {
        let hit = self.matchers.is_match(name);
        if self.negated { !hit } else { hit }
    }
}
```

`*`（既定値）は「すべて表示」に短絡（`is_match` を呼ばずに即 `true`）。

---

## サブフォルダ再帰仕様

ツールバーの「サブフォルダ再帰」チェックボックスと「深さ」数値入力で、リネーム対象に含めるサブフォルダの範囲を制御する。

### セマンティクス

| `recursive` | `depth` | 動作 |
|---|---|---|
| `false` | （無視） | 現フォルダのみ。サブフォルダには立ち入らない |
| `true` | `0` | **無制限**。すべての深さのサブフォルダを走査 |
| `true` | `1` | 直下のサブフォルダのみ（1 階層下） |
| `true` | `N` | N 階層下まで |

UI は `recursive=false` のとき「深さ」入力を disabled にする。値自体は保持して再有効化時に復元する。

### Rust 実装方針

`walkdir::WalkDir` の `max_depth(usize)` を利用する。`walkdir` の `depth` は「root を 0、直下を 1」として数えるため、Naire の指定値とは 1 つずれる。換算式は以下:

```rust
// recursive=false → 現フォルダのみ
// recursive=true, depth=0 → 無制限
// recursive=true, depth=N → N 階層下まで
let walker = WalkDir::new(&folder).min_depth(1);  // root 自身は除外
let walker = if !recursive {
    walker.max_depth(1)              // 直下ファイルのみ
} else if depth == 0 {
    walker                           // 無制限
} else {
    walker.max_depth(depth as usize) // N 階層下まで
};
```

---

## 連番カウンタ仕様

中央ペイン下部の「連番」設定パネルで、advanced 全モード（`高度な` / `マクロ` 内 advanced ステップ）の `?` `??` `???` `????` 変数のカウンタ動作を制御する。

### 設定項目

| 項目 | 型 | 既定 | 制約 |
|---|---|---|---|
| 開始値 (`start`) | u64 | `0` | 0 以上（負の値は不可） |
| ステップ (`step`) | u64 | `1` | 1 以上（負の値・0 は不可） |
| フォルダごとにリセット (`reset_per_folder`) | bool | `true` | — |
| 進数 (`numbering`) | enum | `"decimal"` | `decimal` / `hex` / `alpha` |

### 進数の動作

| 進数 | 値 → 文字列変換 | 例（??, start=0, step=1） |
|---|---|---|
| `decimal` (10進) | 通常の 10 進表記 | `00, 01, 02, ..., 99, 100, ...` |
| `hex` (16進) | **大文字** A-F、`0` パディング | `00, 01, ..., 0F, 10, ..., FF, 100, ...` |
| `alpha` (英大文字 Excel 列名) | A=0, B=1, ..., Z=25, AA=26, ..., ZZ=701, AAA=702, ... | `AA, AB, ..., AZ, BA, ..., ZZ, AAA, ...` |

**桁指定（`?` の数）の意味**: 最小幅。指定桁未満は埋め文字で先頭埋め。
- `decimal` / `hex` は `0` で埋める
- `alpha` は `A` で埋める（例: `??` で start=0 → `AA`、start=25 → `AZ`）

桁を超える値は自然に拡張される（例: `??` で値 100 → `100`、`alpha` で値 26 → `AA`）。

### フォルダごとにリセットの挙動

`recursive=true` 時、各フォルダに進入するタイミングでカウンタを `start` に戻す。`reset_per_folder=false` の場合はバッチ全体で 1 つの連続カウンタを使う。

例: `folder1/{a, b}, folder2/{c, d}` を `??` で番号付け
- `reset_per_folder=true`:  `a→00, b→01, c→00, d→01`
- `reset_per_folder=false`: `a→00, b→01, c→02, d→03`

### 定型操作との関係

定型 #1（文字列+連番）/ #5（連番を先頭に追加）/ #6（連番を末尾に追加）/ #11（フォルダ名+連番）は op 内に独自の `start` / `step` / `digits` を持つ。**定型ステップでは op の値を優先**し、グローバル `SequenceConfig` は使用しない。

ただし `numbering` と `reset_per_folder` はグローバル値を共有する（定型 op が独自の進数を持たないため）。

### Rust 実装方針

```rust
// src-tauri/src/rename/sequence.rs

pub fn format_seq(value: u64, digits: usize, numbering: Numbering) -> String {
    match numbering {
        Numbering::Decimal => format!("{:0>width$}", value, width = digits),
        Numbering::Hex     => format!("{:0>width$X}", value, width = digits),
        Numbering::Alpha   => format_alpha(value, digits),
    }
}

/// Excel 列名スタイル（A=0, B=1, ..., Z=25, AA=26, AB=27, ..., AAA=702）
/// 0-indexed bijective base-26
fn format_alpha(mut value: u64, digits: usize) -> String {
    let mut chars = Vec::new();
    loop {
        chars.push((b'A' + (value % 26) as u8) as char);
        if value < 26 { break; }
        value = value / 26 - 1;
    }
    let mut s: String = chars.iter().rev().collect();
    while s.len() < digits {
        s.insert(0, 'A');
    }
    s
}
```

---

## サポートボタン仕様

検索 / 置換 入力フィールドそれぞれの右肩に「サポート▶」ボタンを設置する。クリックでドロップダウンメニューが表示され、項目選択時に **対応するフィールドのカーソル位置にテキストを挿入**（選択範囲があれば置換）する。

### 型定義

```typescript
export interface SupportItem {
  label: string;    // 表示ラベル（例: "任意の1文字"）
  insert: string;   // 挿入テキスト（例: "?"）
  hint?:  string;   // 右端のヒント表示（変数記号など）
}

export interface SupportItemGroup {
  items: SupportItem[];   // グループ間に区切り線が入る
}

export type SupportMenu = SupportItemGroup[];

export type SupportContext =
  | "wildcard.search" | "wildcard.replace"
  | "regex.search"    | "regex.replace"
  | "char.search"     | "char.replace";
```

### カーソル位置挿入ロジック

```typescript
// src/components/SupportButton/insert-at-cursor.ts

export function insertAtCursor(
  input: HTMLInputElement | HTMLTextAreaElement,
  text:  string,
): { value: string; cursor: number } {
  const start = input.selectionStart ?? input.value.length;
  const end   = input.selectionEnd   ?? input.value.length;
  const value = input.value.slice(0, start) + text + input.value.slice(end);
  const cursor = start + text.length;
  return { value, cursor };
}
```

呼び出し側で setState 後に `requestAnimationFrame(() => input.setSelectionRange(cursor, cursor))` を実行し、フォーカスとカーソル位置を復元する。

### メニュー項目定義

#### `wildcard.search`

| ラベル | 挿入 |
|---|---|
| 任意の1文字 | `?` |
| 任意の文字列 | `*` |

#### `regex.search`

| グループ | ラベル | 挿入 |
|---|---|---|
| 標準 | 任意の1文字 | `.` |
|  | 任意の文字列 | `.*` |
| 文字クラス | A から Z の範囲内 | `[A-Z]` |
|  | A から Z の範囲外 | `[^A-Z]` |
|  | 数字 | `\d` |
|  | 数字以外 | `\D` |
|  | 単語 | `\w` |
|  | 単語以外 | `\W` |
|  | 空白 | `\s` |
|  | 空白以外 | `\S` |
| メタ文字 | ピリオド | `\.` |
|  | 先頭 | `^` |
|  | 末尾 | `$` |
|  | または | `\|` |
|  | 直前の1文字、または空文字 | `?` |
|  | 0回以上の繰り返し | `*` |
|  | 1回以上の繰り返し | `+` |
|  | m回の繰り返し | `{m}` |
|  | m回以上の繰り返し | `{m,}` |
|  | m回以上n回以内の繰り返し | `{m,n}` |
| グループ | グループ化・キャプチャ | `()` |

#### `wildcard.replace` / `regex.replace`（共通）

| グループ | ラベル | 挿入 |
|---|---|---|
| ファイル | 現在の名前 | `\0` |
|  | 現在のファイルタイトル | `\t` |
|  | 現在の拡張子 | `\e` |
|  | フォルダ名 | `\f` |
|  | 親フォルダ名 | `\F` |
|  | ファイルサイズ（桁区切りあり） | `\;` |
|  | ファイルサイズ（桁区切りなし） | `\:` |
| 連番 | 1桁連番 | `?` |
|  | 2桁連番 | `??` |
|  | 3桁連番 | `???` |
|  | 4桁連番 | `????` |
| 日時（ファイル mtime） | ファイル日付（4桁西暦+月+日） | `\Y\m\d` |
|  | ファイル日付（2桁西暦+月+日） | `\y\m\d` |
|  | ファイル時刻（24時+分+秒） | `\H\M\S` |
|  | ファイル時刻（12時+分+秒） | `\I\M\S` |
|  | ファイル時刻（午前午後+時+分+秒） | `\p\H\M\S` |
|  | 曜日（省略名） | `\a` |
|  | 曜日（正式名） | `\A` |
|  | 月名（省略名） | `\b` |
|  | 月名（正式名） | `\B` |
|  | 先行ゼロ削除（次の日時変数に適用） | `\#` |
| キャプチャ | グループ1にマッチ | `\1` |
|  | グループ2にマッチ | `\2` |
|  | グループ3にマッチ | `\3` |
|  | グループ4にマッチ | `\4` |
|  | グループ5にマッチ | `\5` |
|  | グループ6にマッチ | `\6` |
|  | グループ7にマッチ | `\7` |
|  | グループ8にマッチ | `\8` |
|  | グループ9にマッチ | `\9` |
| 大文字小文字制御 | 次にくる1文字を大文字 | `\u` |
|  | 次にくる文字から `\E` までを大文字 | `\U` |
|  | 次にくる1文字を小文字 | `\l` |
|  | 次にくる文字から `\E` までを小文字 | `\L` |
|  | `\U` `\L` の効果終了 | `\E` |
| マクロ専用 | マクロ開始時の元ファイル名 | `\orig` |

> マクロ専用グループはマクロ編集中の advanced ステップでのみ表示する（`replace` メニューがマクロコンテキスト下で開かれた場合）。非マクロ時は出さない。

#### `char.search` / `char.replace`（共通）

| グループ | ラベル | 挿入 |
|---|---|---|
| 英字 | 小文字 | `a-z` |
|  | 大文字 | `A-Z` |
|  | 全角小文字 | `ａ-ｚ` |
|  | 全角大文字 | `Ａ-Ｚ` |
| 仮名 | ひらがな | `あ-ん` |
|  | カタカナ | `ア-ン` |
| 数字 | 数字 | `0-9` |
|  | 全角数字 | `０-９` |
|  | 漢数字 | `〇一二三四五六七八九十` |
|  | 大字表記 | `〇壱弐参四五六七八九拾` |
| ローマ数字 | ローマ数字[小] | `0,i,ii,iii,iv,v,vi,vii,viii,ix,x` |
|  | ローマ数字[大] | `0,I,II,III,IV,V,VI,VII,VIII,IX,X` |

文字変換モードは範囲記法（`a-z`）と列挙（カンマ区切り）の両方を受け付けるよう実装する（詳細は後続で仕様化）。

---

### 正規表現エンジン — Boost.Regex (FR) との互換性

Naire は Rust の `regex` クレートを採用する。FR は Boost.Regex を使用しており、以下の構文に違いがある。

| 機能 | Boost.Regex (FR) | Rust `regex` (Naire) | Naire での対応 |
|---|---|---|---|
| `\<` 語の先頭位置 | ✅ | ❌ | `\b` で近似（前後の文字が語/非語の境界）|
| `\>` 語の末尾位置 | ✅ | ❌ | `\b` で近似 |
| 後方参照（パターン内）`(\d)\1` | ✅ | ❌ | **非対応**。マクロの複数ステップで分解する |
| 先読み・後読み `(?=)` `(?!)` `(?<=)` `(?<!)` | ✅ | ❌ | **非対応**。マクロの複数ステップで分解する |
| `\b` `\B` 語境界 / 非語境界 | ✅ | ✅ | 互換 |
| `\A` `\Z` 文字列先頭 / 末尾 | ✅ | ✅ | 互換 |
| 最小マッチ `*?` `+?` `??` `{m,n}?` | ✅ | ✅ | 互換 |
| 非キャプチャグループ `(?:...)` | ✅ | ✅ | 互換 |
| 標準文字クラス `\s \S \d \D \w \W` | ✅ | ✅ | 互換 |

> 後方参照と先読み・後読みの非対応は Rust `regex` クレートの設計（線形時間保証・ReDoS 耐性）に起因する。Naire では複雑なマッチをマクロの順次ステップに分解する方針で代替する。

---

## マクロシステム仕様

> **参考**: 旧 Flexible Renamer の「プリセット」機能（検索/置換ペアの保存スロット）はマクロの 1 ステップで完全に代替できるため、Naire では独立機能としては実装せずマクロに統合する。Naire の「定型」は Flexible Renamer の「定型リネームメニュー」に相当する。

### 概念

マクロ = 順序付きステップリスト。各ステップは「定型 1 個」または「高度なリネーム操作 1 個」。
`macro_runner.rs` がステップを `fold` で左から右へ順次適用する。

**`\orig` 変数**：fold は毎ステップでファイル名を上書きするため、中間ステップで元のファイル名を参照できるよう
`MacroContext` に `original_name`（マクロ開始時点のファイル名）を保持する。
置換文字列の `\orig` はこの値に展開される。

```rust
// src-tauri/src/rename/macro_runner.rs

/// バッチ実行開始時にファイル毎に1回だけ構築する読み取り専用情報。
/// 日時系変数（\Y \m \d 等）はすべてこの mtime ベースで展開される。
pub struct FileContext {
    pub path:     PathBuf,            // フルパス（\f \F の解決に使用）
    pub size:     u64,                // \; \: のため Metadata::len()
    pub mtime:    DateTime<Local>,    // \Y \y \m \d \H \I \M \S \p \a \A \b \B
    pub seq:      u64,                // 連番カウンタ（? ?? ??? ????）
}

pub struct MacroContext<'a> {
    pub file_ctx:      &'a FileContext,  // パス・サイズ・mtime・連番
    pub original_name: String,           // マクロ開始時点のファイル名（\orig で参照）
}

pub fn run_macro(filename: &str, steps: &[RenameStepDto], file_ctx: &FileContext) -> String {
    let ctx = MacroContext {
        file_ctx,
        original_name: filename.to_string(),
    };
    steps.iter().fold(filename.to_string(), |name, step| {
        apply_step(&name, step, &ctx)
    })
}

// variables.rs で \orig を展開
fn expand_variables(template: &str, current: &str, ctx: &MacroContext) -> String {
    template
        .replace(r"\orig", &ctx.original_name)
        // ... 既存の \0 \t \e \f 等の展開
        // 日時系（\Y \m \d 等）は ctx.file_ctx.mtime を chrono で書式化
}
```

### MacroEditor.tsx の UI

```
┌──────────────────────────────────────────┐
│ マクロ名: [コミック整理________________]  │
│                                          │
│ ≡ Step 1  [定型▼] 先頭末尾フラグ除去   [×] │
│ ≡ Step 2  [定型▼] 先頭半角スペース除去 [×] │
│ ≡ Step 3  [正規表現  ▼]                   [×] │
│           検索: ^\[(.+)\] (.*)               │
│           置換: \f_\1_001                    │
│ ≡ Step 4  [定型▼] 全角→半角（拡張子除く）[×] │
│                                          │
│ [+ ステップ追加]    [保存]    [削除]      │
└──────────────────────────────────────────┘
```

- ステップ追加: モード選択（定型 / ワイルドカード / 正規表現 / 文字変換）→ 選択後にパラメータが即展開
- 並び替え: `dnd-kit` による D&D
- 保存: `AppConfig.macros` に追記して `tauri-plugin-store` へ永続化

### MacroList.tsx の UI（インポート／エクスポート）

```
┌──────────────────────────────────────────┐
│ 保存済みマクロ              [＋新規] [↑インポート] │
│                                          │
│ > コミック整理     [実行] [編集] [↓] [×] │
│ > 作者ソートキー追加 [実行] [編集] [↓] [×] │
│                                          │
│ ※ [↓] = このマクロを JSON エクスポート  │
│    [↑インポート] = JSON ファイルから追加 │
└──────────────────────────────────────────┘
```

---

## マクロ JSON フォーマット仕様

### ファイル名規則

| 種別 | ファイル名 |
|---|---|
| 単一エクスポート | `naire_macro_{マクロ名}.json` |
| 複数エクスポート（全件） | `naire_macros.json` |

### JSON 構造

単一マクロ（`Macro` オブジェクト）と配列（`Macro[]`）の**どちらもインポート可能**。
Claude から渡す際は単一オブジェクト形式を推奨。

```json
{
  "id": "uuid-v4",
  "name": "作者ソートキー追加",
  "created_at": "2026-05-10T20:00:00+09:00",
  "updated_at": "2026-05-10T20:00:00+09:00",
  "steps": [
    {
      "kind": "regex",
      "search": "^\\[(.)(.*?)\\].*",
      "replace": "\\l\\1"
    },
    {
      "kind": "builtin",
      "op": { "type": "kana_convert", "conversion": "kata_to_hira", "skip_ext": false }
    },
    {
      "kind": "builtin",
      "op": { "type": "remove_voiced_mark", "skip_ext": false }
    },
    {
      "kind": "regex",
      "search": "^(.*)$",
      "replace": "\\1 \\orig"
    }
  ]
}
```

### インポート処理（`macro_io.rs`）

```rust
// src-tauri/src/macro_io.rs

use tauri_plugin_dialog::DialogExt;

/// インポート：単一オブジェクト・配列どちらも受け付ける
pub async fn import_macros_from_dialog(app: &tauri::AppHandle) -> Result<Vec<Macro>, String> {
    let path = app.dialog()
        .file()
        .add_filter("Naire Macro", &["json"])
        .blocking_pick_file()
        .ok_or("キャンセルされました")?;

    let text = std::fs::read_to_string(path.path).map_err(|e| e.to_string())?;

    // 配列・単一オブジェクト両対応
    let macros: Vec<Macro> = if text.trim_start().starts_with('[') {
        serde_json::from_str(&text).map_err(|e| e.to_string())?
    } else {
        let m: Macro = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        vec![m]
    };

    // id の重複を避けるため、インポート時に id を再生成
    Ok(macros.into_iter().map(|mut m| {
        m.id = uuid::Uuid::new_v4().to_string();
        m
    }).collect())
}

/// エクスポート：単一 or 複数を保存ダイアログで書き出す
pub async fn export_macros_to_dialog(
    app:    &tauri::AppHandle,
    macros: Vec<Macro>,
) -> Result<(), String> {
    let default_name = if macros.len() == 1 {
        format!("naire_macro_{}.json", macros[0].name)
    } else {
        "naire_macros.json".to_string()
    };

    let path = app.dialog()
        .file()
        .add_filter("Naire Macro", &["json"])
        .set_file_name(&default_name)
        .blocking_save_file()
        .ok_or("キャンセルされました")?;

    let json = serde_json::to_string_pretty(
        if macros.len() == 1 { &macros[0] as &dyn serde::Serialize }
        else                  { &macros     as &dyn serde::Serialize }
    ).map_err(|e| e.to_string())?;

    std::fs::write(path.path, json).map_err(|e| e.to_string())
}
```

> `uuid` クレートを `Cargo.toml` に追加: `uuid = { version = "1", features = ["v4"] }`

### マクロ活用例：作者名先頭文字をソートキーとして先頭に追加

`[ガタタン研究会] 芦別名物「ガタタン」のすべて`
→ `か [ガタタン研究会] 芦別名物「ガタタン」のすべて`

```
Step 1  [正規表現]  検索: ^\[(.)(.*)\].*   置換: \1
        → ファイル名が先頭1文字だけになる（例: ガ）
        ※ \orig には元の名前が保持されたまま

Step 2  [定型] カタカナ → ひらがな（拡張子を除く）
        → が

Step 3  [定型] 濁音・半濁音の除去
        → か

Step 4  [正規表現]  検索: ^(.*)$   置換: \1 \orig
        → か [ガタタン研究会] 芦別名物「ガタタン」のすべて
```

**アルファベット先頭の場合の補足**

カタカナ同様に `Step 2` の前段に条件分岐が必要だが、正規表現の OR マッチで
`Step 1` の置換を `\l\1` とすることで英字の場合も半角小文字に変換できる。

```
Step 1  [正規表現]  検索: ^\[(.)(.*)\].*   置換: \l\1
        → 英字（A→a）・カタカナはそのまま（\l は ASCII のみに作用）
```

---

## UNDO 仕様

| 項目 | 仕様 |
|---|---|
| スタック単位 | **処理（バッチ操作）1 回分** |
| 1件の内容 | `execute_rename` 1 回で変更した全ファイルの `{old_path, new_path}` ペア配列 |
| スタック上限 | **20 処理**（1処理に何千ファイルが含まれていても 1 件と数える） |
| UNDO 操作 | スタック最上位の `RenameRecord` を pop → `undo_rename(record)` で逆適用 |
| REDO | **実装しない** |
| 永続化 | **しない**（アプリ終了時にスタックはクリア） |

---

## 定型一覧（46 種）

### 連番・文字列の追加（11 種）

| # | 定型名 | パラメータ |
|---|---|---|
| 1 | 文字列（日時）＋ 連番 | prefix、suffix、桁数、開始値、ステップ |
| 2 | 連番 ＋ 文字列（日時） | 同上 |
| 3 | 連番（日時）を先頭から n 文字に | n |
| 4 | 文字列（日時）を末尾から n 文字に | n |
| 5 | 連番を先頭に追加 | 桁数・開始値・ステップ |
| 6 | 連番を末尾に追加 | 桁数・開始値・ステップ |
| 7 | 日時 | 日時書式文字列 |
| 8 | 日時を先頭に追加 | 日時書式文字列 |
| 9 | 日時を末尾に追加 | 日時書式文字列 |
| 10 | フォルダ名を先頭に追加 | なし |
| 11 | フォルダ名 ＋ 連番 | 桁数・開始値・ステップ |

### 数字・文字列の削除（14 種）

| # | 定型名 | パラメータ |
|---|---|---|
| 12 | 先頭から n 文字目より n 文字を削除 | offset, count |
| 13 | 末尾から n 文字目より n 文字を削除 | offset, count |
| 14 | 先頭から n 文字目より前を削除 | n |
| 15 | 末尾から n 文字目より前を削除 | n |
| 16 | "コピー（数字）"〜" を削除 | なし |
| 17 | "ーコピー（数字）" を削除（Vista） | なし |
| 18 | 括弧とその中身を削除 | なし |
| 19 | 【数字】または（数字）を削除 | なし |
| 20 | 【数字】を削除 | なし |
| 21 | 〔数字〕を削除 | なし |
| 22 | （数字）を削除 | なし |
| 23 | "^へのショートカット" を削除 | なし |
| 24 | "ーショートカット" を削除（Vista） | なし |
| 25 | 8.3 形式化 | なし |

### 文字種の変換（**11 種**）

| # | 定型名 | パラメータ | 実装 |
|---|---|---|---|
| 26 | 語頭を大文字 | 拡張子を除くか | `str::to_uppercase` |
| 27 | 小文字 → 大文字 | 拡張子を除くか | 同上 |
| 28 | 大文字 → 小文字 | 拡張子を除くか | `str::to_lowercase` |
| 29 | 半角 → 全角（拡張子を除く） | なし | Unicode +0xFEE0 |
| 30 | 全角 → 半角（拡張子を除く） | なし | Unicode -0xFEE0 |
| 31 | ひらがな → カタカナ（拡張子を除く） | なし | Unicode +0x60 |
| 32 | カタカナ → ひらがな（拡張子を除く） | なし | Unicode -0x60 |
| 33 | ダイアクリティカルマークをクリア | なし | NFD → Mn 除去 → NFC |
| 34 | 半角カナ → 全角カナ | なし | lookup table |
| 35 | 全角カナ → 半角カナ | なし | lookup table |
| **36** | **濁音・半濁音の除去** | 拡張子を除くか | **NFD → U+3099/U+309A 除去 → NFC** |

> **定型 #36 の主な用途**：マクロの中間ステップとして使用する。
> 例）`作者名ソートキー追加マクロ`：正規表現で先頭1文字抽出 → カタカナ→ひらがな → 濁音除去 → `\orig` で元名に先頭付加。
> 詳細はマクロシステム仕様の活用例を参照。

### 文字列の置換（1 種）

| # | 定型名 | パラメータ |
|---|---|---|
| 37 | 文字列置換 | 検索文字列・置換文字列 |

### 数値の整理（4 種）

| # | 定型名 | パラメータ |
|---|---|---|
| 38 | 先頭から n 番目の数値の桁合わせ | n、桁数 |
| 39 | 末尾から n 番目の数値の桁合わせ | n、桁数 |
| 40 | 先頭から n 番目の数値を増減 | n、増減値 |
| 41 | 末尾から n 番目の数値を増減 | n、増減値 |

### 拡張子の変換（5 種）

| # | 定型名 | パラメータ |
|---|---|---|
| 42 | 拡張子を大文字 | なし |
| 43 | 拡張子を小文字 | なし |
| 44 | 拡張子を削除 | なし |
| 45 | 拡張子を追加 | 追加する拡張子文字列 |
| 46 | 拡張子を置換 | 新しい拡張子文字列 |

---

## 設定永続化

- ストレージ: `tauri-plugin-store`（`config.json`）
- 保存タイミング: フォルダ変更時・ウィンドウリサイズ時・マクロ保存/削除時・フィルタ確定時・連番設定変更時
- 保存項目: `AppConfig`（型定義参照）

---

## 実装優先順位

1. **骨格**: フォルダツリー + ファイル一覧表示 + プレビューパネル
2. **表示フィルタ**（list_entries で絞り込み）
3. **正規表現モード**（コア機能・最頻用途）
4. **サポートボタン**（検索/置換のスニペット挿入。最初は regex の最低限から）
5. **連番カウンタ**（10 進数のみ → 後続で 16 進数 / 英大文字を追加）
6. **UNDO 機構**
7. **定型実装**（カテゴリ順に順次追加）
8. **ワイルドカードモード**
9. **文字変換モード**
10. **マクロシステム**（`\orig` 変数含む）
11. **マクロ JSON インポート／エクスポート**
12. **設定永続化**（最終仕上げ）

---

## CLAUDE.md

```markdown
# CLAUDE.md — Naire

## プロジェクト概要
Flexible Renamer 後継の一括リネームツール（Tauri v2 + React + TypeScript）

## 絶対に実装しないこと
- タグリネーム（ID3 / EXIF）
- ファイル属性・タイムスタンプ変更
- 連番オブジェクト生成
- REDO

## 命名規則
- Rust: snake_case
- TypeScript: camelCase（型は PascalCase）
- コンポーネント: PascalCase

## アーキテクチャ原則
- プレビュー計算は必ず Rust 側（commands.rs → preview_rename）で行う
  フロントエンドで文字列処理を行わない
- 文字変換は src-tauri/src/rename/convert.rs に集約する
- 表示フィルタは src-tauri/src/filter.rs に集約。`globset` を使用しセミコロン区切り＋ `^(...)` 否定をサポート
- フィルタ適用は list_entries / preview_rename / execute_rename の Rust 側で行い、フロントには絞り込み済みリストのみ返す
- 連番フォーマッタは src-tauri/src/rename/sequence.rs に集約。10進/16進大文字/英大文字 Excel 列名スタイルの 3 種をサポート
- `start` / `step` は 0 以上 / 1 以上に制約（負の値・0 ステップは UI 側でも入力ガード）
- グローバル `SequenceConfig` は advanced 系ステップのみに適用。定型 op 内 start/step/digits は op 側を優先する
- 検索/置換 入力欄のサポートボタンは src/components/SupportButton/ に集約。項目定義は support-items.ts、挿入ロジックは insert-at-cursor.ts に分離
- サポートメニュー項目はモード × フィールドの 6 コンテキストごとに定義（wildcard.search / .replace, regex.search / .replace, char.search / .replace）
- マクロ専用変数 `\orig` のサポート項目はマクロ編集時の advanced ステップでのみ表示する
- 濁音・半濁音除去は NFD → U+3099/U+309A 除去 → NFC（unicode-normalization クレート）
- `\orig` 変数はマクロ専用。`MacroContext::original_name` に保持し `variables.rs` で展開する
- 日時系変数（`\Y \y \m \d \H \I \M \S \p \a \A \b \B`）は **ファイル mtime ベース**（`std::fs::Metadata::modified()` を `chrono::DateTime<Local>` に変換）。現在日時ではない
- ロケール依存変数（`\a \A \b \B \p`）は `chrono::format::Locale` で OS ロケールから決定
- UNDO スタック上限は 20 件（処理単位）。永続化しない。
- マクロの fold 実行は macro_runner.rs の run_macro に集約する
- マクロ IO（インポート／エクスポート）は macro_io.rs に集約する
- インポート時は id を必ず再生成する（重複防止）
- エクスポートは単一マクロ＝オブジェクト、複数＝配列で書き分ける

## Cargo.toml 主要依存
regex = "1"
walkdir = "2"
globset = "0.4"  # 表示フィルタ（セミコロン区切りグロブ）
chrono = { version = "0.4", features = ["serde", "unstable-locales"] }  # \a \A \b \B \p のロケール対応
unicode-normalization = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4"] }

## tauri.conf.json plugins
tauri-plugin-store
tauri-plugin-dialog
```
