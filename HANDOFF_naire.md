# HANDOFF — Naire

ファイル・フォルダ一括リネームツール。Flexible Renamer（最終更新 2011 年）の後継として自作。

- **リポジトリ名**: `naire`
- **productName**: `Naire`
- **bundle identifier**: `com.polarissolutions.naire`（`tauri.conf.json` で設定）

---

## 概要

Windows / macOS 対応の一括リネームツール。
正規表現・ワイルドカード・プリセット・簡易マクロに対応し、
日本語ファイル名操作（全半角・仮名変換・濁音除去）を完全サポートする。

**スコープ内**
- 正規表現・ワイルドカード・プリセット・簡易マクロによるリネーム
- 日本語ファイル名操作（全半角・仮名変換・濁音/半濁音除去）の完全サポート
- サブフォルダ再帰処理・リアルタイムプレビュー・複数段 UNDO（最大 20 処理）
- マクロの JSON インポート／エクスポート（Claude 連携用途を含む）

**スコープ外（実装しない）**
- タグリネーム（ID3 / EXIF）
- ファイル属性・タイムスタンプ変更
- 連番オブジェクト生成
- REDO
- ネットワークドライブのブラウズ

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
│  [● ファイル] [○ フォルダ]    ☑ サブフォルダ再帰         [歯車] │  ← ツールバー
├────────────┬──────────────────────┬──────────────────────────────┤
│            │ [プリセット][高度な][マクロ]                        │
│  フォルダ   ├──────────────────────┼──────────────────────────────┤
│  ツリー    │                      │  現在の名前       新しい名前  │
│  （前回の  │  設定パネル          │  ─────────────────────────── │
│  パスを    │  （モード別 UI）      │  file_a.jpg  →  img_001.jpg  │
│  記憶）    │                      │  file_b.jpg  →  img_002.jpg  │
│            │                      │  ...                         │
├────────────┴──────────────────────┴──────────────────────────────┤
│   [リネーム実行]     [元に戻す Ctrl+Z]     [クリア]              │
└─────────────────────────────────────────────────────────────────┘
```

---

## 型定義（TypeScript）

### `src/types/rename.ts`

```typescript
// ── ターゲット ──────────────────────────────────────────────────
export type TargetType = "file" | "folder";

// ── リネームステップ（Union）───────────────────────────────────
export type RenameStep =
  | { kind: "preset";       op: PresetOp }
  | { kind: "wildcard";     search: string; replace: string }
  | { kind: "regex";        search: string; replace: string }
  | { kind: "char_convert"; from: string;   to: string };

// ── プリセット操作 ────────────────────────────────────────────
export type PresetOp =
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

// ── アプリ設定 ───────────────────────────────────────────────
export interface AppConfig {
  last_folder: string;
  last_mode:   "preset" | "advanced" | "macro";
  last_target: TargetType;
  recursive:   boolean;
  window:      { width: number; height: number; x?: number; y?: number };
  macros:      Macro[];
}
```

---

## コンポーネント構成

```
src/
├── App.tsx
├── components/
│   ├── Toolbar.tsx               # ターゲット切り替え・再帰チェックボックス
│   ├── FolderTree.tsx            # 左ペイン：フォルダツリー（前回パス記憶）
│   ├── ModePanel/
│   │   ├── ModePanel.tsx         # 中央ペイン：タブ切り替え
│   │   ├── preset/
│   │   │   ├── PresetMenu.tsx    # カテゴリ折りたたみツリー
│   │   │   └── PresetParams.tsx  # 選択プリセットのパラメータフォーム（インライン）
│   │   ├── advanced/
│   │   │   ├── WildcardMode.tsx
│   │   │   ├── RegexMode.tsx
│   │   │   └── CharConvertMode.tsx
│   │   └── macro/
│   │       ├── MacroEditor.tsx   # ステップリスト（dnd-kit で D&D 並べ替え）
│   │       └── MacroList.tsx     # 保存済みマクロ一覧
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
) -> Result<Vec<PreviewItem>, String>

/// リネーム実行（UNDO 用レコードを返す）
#[tauri::command]
pub async fn execute_rename(
    folder:    String,
    steps:     Vec<RenameStepDto>,
    target:    TargetType,
    recursive: bool,
) -> Result<RenameRecord, String>

/// UNDO（RenameRecord の逆方向リネームをまとめて実行）
#[tauri::command]
pub async fn undo_rename(record: RenameRecord) -> Result<(), String>

/// フォルダ内エントリ一覧取得
#[tauri::command]
pub async fn list_entries(
    folder:    String,
    target:    TargetType,
    recursive: bool,
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
├── macro_io.rs           # マクロ JSON インポート／エクスポート
└── rename/
    ├── mod.rs            # エントリポイント・ステップ dispatch
    ├── preset.rs         # プリセット処理（全 46 種）
    ├── advanced.rs       # ワイルドカード・正規表現処理
    ├── convert.rs        # 文字変換（仮名・全半角・濁音除去）
    ├── macro_runner.rs   # マクロ：ステップを fold で順次適用
    ├── variables.rs      # 置換変数展開（\f \e \t \Y\m\d \orig 等）
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

| 変数 | 意味 | Rust 実装 |
|------|------|-----------|
| `\0` | 現在のファイル名（拡張子含む） | そのまま |
| `\t` | ファイルタイトル（拡張子除く） | `Path::file_stem()` |
| `\e` | 現在の拡張子（ドット含む） | `Path::extension()` に `.` を付加 |
| `\f` | 所属フォルダ名 | `path.parent()?.file_name()` |
| `\F` | 親フォルダ名 | `path.parent()?.parent()?.file_name()` |
| `?` | 1桁連番 | カウンタ |
| `??` | 2桁連番 | カウンタ（`{:02}`） |
| `???` | 3桁連番 | カウンタ（`{:03}`） |
| `????` | 4桁連番 | カウンタ（`{:04}`） |
| `\Y` / `\y` | 4桁 / 2桁西暦年 | `chrono::Local::now()` |
| `\m` | 月（2桁） | 同上 |
| `\d` | 日（2桁） | 同上 ※正規表現の `\d`（数字）と別物 |
| `\H` `\M` `\S` | 時・分・秒 | 同上 |
| `\p` | 午前 / 午後 | 同上 |
| `\1`〜`\9` | キャプチャグループ | regex の `${1}`〜`${9}` |
| `\u` | 次の 1 文字を大文字 | 後処理で展開 |
| `\U` | `\E` まで大文字 | 後処理で展開 |
| `\l` | 次の 1 文字を小文字 | 後処理で展開 |
| `\L` | `\E` まで小文字 | 後処理で展開 |
| `\E` | `\U` `\L` の効果終了 | 後処理で展開 |
| `\orig` | **マクロ開始時点の元ファイル名**（マクロ内専用） | `MacroContext::original` |

> `\orig` はマクロのステップ内でのみ有効。非マクロモード（高度なリネーム単体）では展開されない。

**ワイルドカード → 正規表現 変換規則**

```
?  →  .      （任意の 1 文字）
*  →  .*     （任意の文字列）
.  →  \.     （ピリオドのエスケープ）
```

---

## マクロシステム仕様

### 概念

マクロ = 順序付きステップリスト。各ステップは「プリセット 1 個」または「高度なリネーム操作 1 個」。
`macro_runner.rs` がステップを `fold` で左から右へ順次適用する。

**`\orig` 変数**：fold は毎ステップでファイル名を上書きするため、中間ステップで元のファイル名を参照できるよう
`MacroContext` に `original_name`（マクロ開始時点のファイル名）を保持する。
置換文字列の `\orig` はこの値に展開される。

```rust
// src-tauri/src/rename/macro_runner.rs

pub struct MacroContext<'a> {
    pub file_ctx:      &'a FileContext,  // パス・日付等の静的情報
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
}
```

### MacroEditor.tsx の UI

```
┌──────────────────────────────────────────┐
│ マクロ名: [コミック整理________________]  │
│                                          │
│ ≡ Step 1  [プリセット▼] 先頭末尾フラグ除去   [×] │
│ ≡ Step 2  [プリセット▼] 先頭半角スペース除去 [×] │
│ ≡ Step 3  [正規表現  ▼]                   [×] │
│           検索: ^\[(.+)\] (.*)               │
│           置換: \f_\1_001                    │
│ ≡ Step 4  [プリセット▼] 全角→半角（拡張子除く）[×] │
│                                          │
│ [+ ステップ追加]    [保存]    [削除]      │
└──────────────────────────────────────────┘
```

- ステップ追加: モード選択（プリセット / ワイルドカード / 正規表現 / 文字変換）→ 選択後にパラメータが即展開
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
      "kind": "preset",
      "op": { "type": "kana_convert", "conversion": "kata_to_hira", "skip_ext": false }
    },
    {
      "kind": "preset",
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

Step 2  [プリセット] カタカナ → ひらがな（拡張子を除く）
        → が

Step 3  [プリセット] 濁音・半濁音の除去
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

## プリセット一覧（45 種）

### 連番・文字列の追加（11 種）

| # | プリセット名 | パラメータ |
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

| # | プリセット名 | パラメータ |
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

| # | プリセット名 | パラメータ | 実装 |
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

> **プリセット #36 の主な用途**：マクロの中間ステップとして使用する。
> 例）`作者名ソートキー追加マクロ`：正規表現で先頭1文字抽出 → カタカナ→ひらがな → 濁音除去 → `\orig` で元名に先頭付加。
> 詳細はマクロシステム仕様の活用例を参照。

### 文字列の置換（1 種）

| # | プリセット名 | パラメータ |
|---|---|---|
| 37 | 文字列置換 | 検索文字列・置換文字列 |

### 数値の整理（4 種）

| # | プリセット名 | パラメータ |
|---|---|---|
| 38 | 先頭から n 番目の数値の桁合わせ | n、桁数 |
| 39 | 末尾から n 番目の数値の桁合わせ | n、桁数 |
| 40 | 先頭から n 番目の数値を増減 | n、増減値 |
| 41 | 末尾から n 番目の数値を増減 | n、増減値 |

### 拡張子の変換（5 種）

| # | プリセット名 | パラメータ |
|---|---|---|
| 42 | 拡張子を大文字 | なし |
| 43 | 拡張子を小文字 | なし |
| 44 | 拡張子を削除 | なし |
| 45 | 拡張子を追加 | 追加する拡張子文字列 |
| 46 | 拡張子を置換 | 新しい拡張子文字列 |

---

## 設定永続化

- ストレージ: `tauri-plugin-store`（`config.json`）
- 保存タイミング: フォルダ変更時・ウィンドウリサイズ時・マクロ保存/削除時
- 保存項目: `AppConfig`（型定義参照）

---

## 実装優先順位

1. **骨格**: フォルダツリー + ファイル一覧表示 + プレビューパネル
2. **正規表現モード**（コア機能・最頻用途）
3. **UNDO 機構**
4. **プリセット実装**（カテゴリ順に順次追加）
5. **ワイルドカードモード**
6. **文字変換モード**
7. **マクロシステム**（`\orig` 変数含む）
8. **マクロ JSON インポート／エクスポート**
9. **設定永続化**（最終仕上げ）

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
- 濁音・半濁音除去は NFD → U+3099/U+309A 除去 → NFC（unicode-normalization クレート）
- `\orig` 変数はマクロ専用。`MacroContext::original_name` に保持し `variables.rs` で展開する
- UNDO スタック上限は 20 件（処理単位）。永続化しない。
- マクロの fold 実行は macro_runner.rs の run_macro に集約する
- マクロ IO（インポート／エクスポート）は macro_io.rs に集約する
- インポート時は id を必ず再生成する（重複防止）
- エクスポートは単一マクロ＝オブジェクト、複数＝配列で書き分ける

## Cargo.toml 主要依存
regex = "1"
walkdir = "2"
chrono = { version = "0.4", features = ["serde"] }
unicode-normalization = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4"] }

## tauri.conf.json plugins
tauri-plugin-store
tauri-plugin-dialog
```
