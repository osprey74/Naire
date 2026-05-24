# Naire v1.1.0 Release Notes

**Release Date:** 2026-05-24

Naire v1.1.0 is a quality-of-life release focused on the folder tree pane. The tree now supports inline rename, shows per-folder content stats, lets you resize the pane, and exposes a right-click menu to send folders to the OS Recycle Bin / Trash. The「再読み込み」action now refreshes both panes in one go.

No breaking changes. Existing settings, macro libraries, and undo stack semantics are unchanged.

---

## English

### Highlights

#### 🆕 Folder tree quality-of-life

The folder tree on the left pane gets four focused improvements:

- **Inline rename** — Double-click a folder name to edit it in place. `Enter` commits, `Esc` cancels, blur commits. Rename failures (permissions, existing target, etc.) keep edit mode active so you can correct the name. Successful renames are pushed onto the same UNDO stack used by file renames — `Ctrl+Z` reverses them.
- **Folder content stats** — Each tree node displays the total file count and recursive byte size next to its name, e.g. `12 / 1.5 MB`. Stats load lazily on first render and are cached per path. Drive roots (`C:\`, `D:\`, ...) are skipped to avoid expensive top-level scans; the rest of the tree fetches in the background.
- **Resizable pane** — A draggable splitter sits between the folder tree and the center panel. Drag to resize between 140 px and 800 px; the chosen width persists in `AppConfig.left_width` and is restored on the next launch.
- **Right-click → ゴミ箱へ移動** — A native context menu on every folder offers "フォルダを削除（ゴミ箱へ移動）". A confirmation dialog precedes the action, and the folder is moved to the OS Recycle Bin (Windows) or Trash (macOS / Linux) via the cross-platform [`trash`](https://crates.io/crates/trash) crate. Files remain restorable from the OS — Naire's UNDO stack intentionally does not duplicate that.

#### 🔄 Unified reload

The `再読み込み` button in the action bar now refreshes the folder tree in addition to the preview. Drive roots are re-enumerated, all currently expanded folders re-fetch their children, and the stats cache is discarded so visible nodes recompute. The expanded set and current selection are preserved.

### Internal changes

- **New Tauri commands**: `get_folder_stats(path)`, `rename_folder(old_path, new_name)`, `move_folder_to_trash(path)`
- **New dependency**: `trash = "5"` for cross-platform recycle bin support
- **`AppConfig.left_width`**: persisted alongside existing column widths and other UI state
- **`FolderTree` props**: new `reloadSignal`, `onFolderRenamed`, `onFolderDeleted`, `onError` callbacks for parent integration

### Compatibility

- `AppConfig` schema: additive field `left_width` (optional, falls back to 220 px default if missing)
- Macro JSON, `RenameRecord`, and Tauri command surface are unchanged for existing commands

### System Requirements

| | Windows | macOS |
| --- | --- | --- |
| OS | Windows 10 or later | macOS 10.15 (Catalina) or later |
| Architecture | x86_64 | x86_64, Apple Silicon (ARM) |

### Migration from v1.0.0

No action required. Existing config files load as-is; the new `left_width` field is written on the next debounced save.

---

## 日本語

### ハイライト

#### 🆕 フォルダツリーペインの操作性向上

左ペインのフォルダツリーに 4 つの改善を加えました:

- **インラインリネーム** — フォルダ名をダブルクリックでその場で編集できます。`Enter` で確定、`Esc` でキャンセル、フォーカスアウトで確定。リネーム失敗（権限不足・既存衝突等）の場合は編集モードを維持して再入力可能です。成功したリネームはファイルリネームと同じ UNDO スタックに積まれ、`Ctrl+Z` で取り消せます
- **フォルダ内容量の表示** — 各ノードにファイル数と再帰集計サイズを表示します（例: `12 / 1.5 MB`）。スタッツは初回レンダリング時に遅延ロードされ、パスごとにキャッシュされます。ドライブルート（`C:\`、`D:\` 等）はトップレベル全域スキャンを避けるためスキップ、それ以外は背景で取得します
- **ペイン幅可変** — フォルダツリーと中央パネルの境界にドラッグハンドル付きのスプリッタを配置。140〜800px の範囲でドラッグして調整可能。選択した幅は `AppConfig.left_width` に保存され次回起動時に復元されます
- **右クリック → ゴミ箱へ移動** — フォルダ上の右クリックで「フォルダを削除（ゴミ箱へ移動）」項目を含むネイティブコンテキストメニューを表示。確認ダイアログを経て、クロスプラットフォームの [`trash`](https://crates.io/crates/trash) クレート経由で OS のゴミ箱（Windows: Recycle Bin / macOS: Trash）へ送ります。復元は OS 側から可能なため、Naire の UNDO スタックには意図的に積みません

#### 🔄 再読み込みボタンの統合

アクションバーの「再読み込み」ボタンが、プレビューに加えてフォルダツリーも更新するようになりました。ドライブルートを再列挙し、現在展開中のフォルダ全ての children を再取得、スタッツキャッシュを破棄して再計算をトリガーします。展開状態と選択中フォルダはそのまま保持されます

### 内部変更

- **新規 Tauri コマンド**: `get_folder_stats(path)`、`rename_folder(old_path, new_name)`、`move_folder_to_trash(path)`
- **新規依存**: `trash = "5"`（クロスプラットフォームのゴミ箱対応）
- **`AppConfig.left_width`**: 既存のカラム幅等と同じフローで永続化
- **`FolderTree` props**: `reloadSignal` / `onFolderRenamed` / `onFolderDeleted` / `onError` を追加して親コンポーネントと連携

### 互換性

- `AppConfig` スキーマ: 追加フィールド `left_width`（任意、欠落時はデフォルト 220px にフォールバック）
- マクロ JSON、`RenameRecord`、既存 Tauri コマンドはすべて変更なし

### 動作環境

| | Windows | macOS |
| --- | --- | --- |
| OS | Windows 10 以降 | macOS 10.15 (Catalina) 以降 |
| アーキテクチャ | x86_64 | x86_64、Apple Silicon (ARM) |

### v1.0.0 からの移行

移行作業は不要です。既存の設定ファイルはそのまま読み込まれ、新規 `left_width` フィールドは次回の自動保存時に書き出されます。
