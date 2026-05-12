# Naire v0.1.0 Release Notes

**Release Date:** 2026-05-12

Naire is a desktop bulk file/folder rename tool, designed as a modern successor to *Flexible Renamer* (last updated 2011). It runs on **Windows** and **macOS** and is built with Tauri v2 + React + TypeScript + Rust.

---

## English

### 🎉 Initial Release

Naire v0.1.0 is the first public release. All 13 planned phases from the HANDOFF spec are implemented.

### Features

- **Folder navigation** — native picker, parent folder shortcut, recursive sub-folder walk with depth limit
- **Display filter** — semicolon-OR glob patterns (`*.jpg;*.png`), negation (`^(*.tmp)`), 7 built-in presets, 10-entry history
- **Live preview** — see the new name beside the original; changed rows are highlighted
- **Row selection** — click / Ctrl+click (toggle) / Shift+click (range); rename only the selected rows when a subset is selected
- **Resizable columns** — drag the column headers, widths persist across restarts
- **Regex mode** — full regex search/replace via the Rust `regex` crate, with capture groups and rich replacement variables
- **Wildcard mode** — `?` `*` translated to anchored regex
- **Character convert mode** — `tr`-style 1:1 mapping with range (`a-z`) and enumeration (`0,i,ii,iii,...`) syntax
- **22 built-in templates** covering 46 HANDOFF presets:
  - Case (capitalize / upper / lower), Kana (hira↔kata, full↔half), Width (full↔half alnum)
  - Diacritics removal and **voiced/half-voiced mark removal** (a Naire-specific feature)
  - Datetime / folder name / sequence insertion, delete patterns, 8.3 truncation
  - Number padding & adjustment, extension add/delete/replace/case
- **Support button** — context-aware dropdown to insert variables, character classes, and presets into search / replace fields
- **Sequence counter** — decimal / hex / alpha (bijective base-26), with per-folder reset
- **UNDO** — last 20 batches, `Ctrl+Z` keyboard shortcut
- **Macro system** — sequence of steps (built-in or advanced), step-by-step execution with `[◀戻す]` / `[ステップ処理▶]` / `[すべて適用]`; `\orig` variable references the step-0 name
- **Macro JSON import / export** — share macros as `.json`, including Claude-generated ones
- **Settings persistence** — folder, mode, filter, filter history, sequence config, column widths, and the entire macro library survive restarts via `tauri-plugin-store`

### Replacement Variables (regex / wildcard replace field)

| Variable | Meaning |
|---|---|
| `\0` | current name (full filename) |
| `\t` | current title (without extension) |
| `\e` | current extension (with leading dot) |
| `\f` | parent folder name |
| `\F` | grandparent folder name |
| `\;` / `\:` | file size (with / without comma separators) |
| `\Y` `\y` `\m` `\d` `\H` `\I` `\M` `\S` | date components from file mtime |
| `\#X` | strip leading zero from the next date variable |
| `\1`–`\9` | regex capture groups |
| `?` `??` `???` `????` | 1–4 digit sequence number |
| `\orig` | original filename at macro step 0 (macros only) |
| `\?` `\\` | literal `?` / `\` |

### Keyboard Shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl + Z` | Undo the last rename batch |

### System Requirements

| | Windows | macOS |
| --- | --- | --- |
| OS | Windows 10 or later | macOS 10.15 (Catalina) or later |
| Architecture | x86_64 | x86_64, Apple Silicon (ARM) |

### Known Limitations

- Window size / position is not yet restored across sessions
- Locale-dependent date variables (`\a \A \b \B \p`) and case-control variables (`\u \U \l \L \E`) are not yet implemented and pass through as literals
- Macro step reordering uses up/down buttons (drag & drop planned)
- Network drives are not supported as browse targets

---

## 日本語

### 🎉 初回リリース

Naire v0.1.0 は、ファイル / フォルダ一括リネームツールの初公開バージョンです。Flexible Renamer（2011 年で更新停止）の精神的後継として設計され、HANDOFF 仕様で計画した全 13 フェーズを実装済みです。**Windows** および **macOS** に対応しています。

### 機能

- **フォルダナビゲーション** — ネイティブ選択ダイアログ、親フォルダへの移動、深さ指定付きサブフォルダ再帰
- **表示フィルタ** — セミコロン OR グロブ (`*.jpg;*.png`)、否定 (`^(*.tmp)`)、7 つの組み込みプリセット、10 件の履歴
- **リアルタイムプレビュー** — 現在の名前と新しい名前を並べて表示、変更行をハイライト
- **行選択** — クリック / Ctrl+クリック（トグル） / Shift+クリック（範囲）。一部選択時は選択行のみリネーム対象
- **列幅リサイズ** — カラムヘッダをドラッグで調整、再起動後も維持
- **正規表現モード** — Rust の `regex` クレートによる検索／置換、キャプチャグループと豊富な置換変数
- **ワイルドカードモード** — `?` `*` をアンカー付き正規表現に変換
- **文字変換モード** — `tr` 風の 1:1 マッピング。範囲 (`a-z`) と列挙 (`0,i,ii,iii,...`) 構文に対応
- **定型 22 種類**（HANDOFF 46 プリセット相当）:
  - 大文字小文字（語頭大文字／大文字／小文字）、仮名変換（ひら↔カナ、全↔半カナ）、全半角変換
  - ダイアクリティカルマーク除去、**濁音・半濁音の除去**（Naire 独自機能）
  - 日時／フォルダ名／連番の追加、決まったパターンの削除、8.3 形式化
  - 数値の桁合わせ・増減、拡張子の追加／削除／置換／大文字小文字変換
- **サポートボタン** — 検索／置換フィールド横のドロップダウンから変数や文字クラス、プリセットをカーソル位置に挿入
- **連番カウンタ** — 10 進 / 16 進 / 英大文字（bijective base-26）、フォルダごとリセット対応
- **UNDO** — 直近 20 バッチを保持、`Ctrl+Z` で巻き戻し
- **マクロシステム** — 定型・高度なリネームステップを順序付きで保存、`[◀戻す]` / `[ステップ処理▶]` / `[すべて適用]` で段階的に実行。`\orig` 変数でマクロ開始時点のファイル名を参照
- **マクロ JSON 入出力** — マクロを `.json` で共有可能（Claude で生成したマクロも直接インポートできる形式）
- **設定永続化** — フォルダ・モード・フィルタ・履歴・連番設定・列幅・マクロ一覧を `tauri-plugin-store` でディスク保存

### 置換変数（正規表現・ワイルドカードの置換フィールド）

| 変数 | 意味 |
|---|---|
| `\0` | 現在の名前（拡張子含む） |
| `\t` | ファイルタイトル（拡張子除く） |
| `\e` | 現在の拡張子（ドット含む） |
| `\f` | 所属フォルダ名 |
| `\F` | 親フォルダ名 |
| `\;` / `\:` | ファイルサイズ（3 桁区切りあり / なし）|
| `\Y` `\y` `\m` `\d` `\H` `\I` `\M` `\S` | ファイル mtime ベースの日付要素 |
| `\#X` | 次の日付変数の先行ゼロを除去 |
| `\1`〜`\9` | 正規表現キャプチャグループ |
| `?` `??` `???` `????` | 1〜4 桁の連番 |
| `\orig` | マクロ開始時点の元ファイル名（マクロ専用） |
| `\?` `\\` | リテラル `?` / `\` |

### キーボードショートカット

| ショートカット | 操作 |
| --- | --- |
| `Ctrl + Z` | 直近のリネームを元に戻す |

### 動作環境

| | Windows | macOS |
| --- | --- | --- |
| OS | Windows 10 以降 | macOS 10.15 (Catalina) 以降 |
| アーキテクチャ | x86_64 | x86_64、Apple Silicon (ARM) |

### 既知の制限

- ウィンドウサイズ・位置の復元は未実装
- ロケール依存日時変数（`\a \A \b \B \p`）と大文字小文字制御変数（`\u \U \l \L \E`）は未実装でリテラル出力されます
- マクロステップの並べ替えは上下ボタン式（D&D 対応は将来予定）
- ネットワークドライブのブラウズは非対応
