# Naire v0.2.0 Release Notes

**Release Date:** 2026-05-13

Naire v0.2.0 resolves all four known limitations listed in v0.1.0 and adds
UI polish for the folder browser, an About dialog, and a drag-and-drop
macro step editor.

---

## English

### Highlights

This release fully closes the four "Known Limitations" listed in v0.1.0:

- Window size / position is now restored across sessions
- Locale-dependent date variables (`\a \A \b \B \p`) and case-control
  variables (`\u \U \l \L \E`) are now implemented
- Macro steps can be reordered by drag and drop
- Network-mounted volumes are reachable from the folder tree on macOS;
  mapped network drives on Windows have always worked

### New Features

#### Folder browser

- **Explorer-style folder tree** in the left pane — lazy-loaded with an
  expand arrow, single-click to select. Auto-expands to your last folder
  on startup. Replaces the old "pick a folder" dialog-only flow (the
  dialog is still available as「場所を選択…」for arbitrary locations).
- **macOS roots**: home folder plus every entry under `/Volumes` — so
  external disks, SMB shares, and AFP mounts appear automatically.
- **Windows roots**: every connected drive letter, including mapped
  network drives.

#### About dialog

- New「アプリについて」button in the toolbar shows the app version,
  copyright, and the Flaticon attribution required by the icon license.
  External links open in the OS browser.

#### Macro editor

- **Drag-and-drop reordering** for macro steps (replaces the up/down
  buttons). Built on `@dnd-kit/sortable` with keyboard support.

#### Window state

- Window position, size, and maximized state are now persisted via
  `tauri-plugin-window-state` and restored on next launch.

#### Replacement variables

- **Locale-dependent date variables** (resolved against your OS locale):
  - `\a` abbreviated weekday name (e.g. `Mon` / `月`)
  - `\A` full weekday name (e.g. `Monday` / `月曜日`)
  - `\b` abbreviated month name (e.g. `May` / `5月`)
  - `\B` full month name (e.g. `May` / `5月`)
  - `\p` AM/PM designation (e.g. `AM` / `午前`)
- **Case-control variables** (compose with capture groups and other
  variables):
  - `\u` uppercase the next single character
  - `\l` lowercase the next single character
  - `\U` ... `\E` uppercase a range
  - `\L` ... `\E` lowercase a range

### UX changes

- The bottom-bar「クリア」button has been renamed to「再読み込み」. Row
  selection is now preserved across the refresh (it was previously
  cleared, which surprised users).

### Internal

- 8 new tests for the locale and case-control variables (84 tests total
  passing).
- New dependencies: `tauri-plugin-window-state`, `tauri-plugin-opener`,
  `sys-locale`.

### System Requirements

| | Windows | macOS |
| --- | --- | --- |
| OS | Windows 10 or later | macOS 10.15 (Catalina) or later |
| Architecture | x86_64 | x86_64, Apple Silicon (ARM) |

### Known Limitations

- Programmatic Windows network enumeration (browsing `\\server\share`
  hosts that are not mapped to a drive letter) is not implemented. Use
  「場所を選択…」to type a UNC path directly.

---

## 日本語

### ハイライト

v0.1.0 で「既知の制限」として掲示していた 4 項目を全て解消したリリースです:

- ウィンドウサイズ・位置が再起動後に復元されるようになりました
- ロケール依存日時変数（`\a \A \b \B \p`）と大文字小文字制御変数（`\u \U \l \L \E`）が利用可能になりました
- マクロステップをドラッグ＆ドロップで並べ替え可能になりました
- macOS のネットワークマウントがフォルダツリーから直接たどれるようになりました（Windows のマップ済みネットワークドライブは従来通り）

### 新機能

#### フォルダブラウザ

- **エクスプローラー風フォルダツリー**を左ペインに搭載。▶/▼ で展開・折りたたみ、フォルダ名クリックで選択。前回開いていたフォルダまで起動時に自動展開します。従来のダイアログ専用フローは置き換わり、ダイアログは「場所を選択…」ボタンとして任意の場所を開く用途に残しています
- **macOS のルート**: ホームフォルダ + `/Volumes` 配下の全エントリ（外部ディスク・SMB・AFP マウントが自動で表示）
- **Windows のルート**: 接続中の全ドライブ（マップ済みネットワークドライブを含む）

#### アプリについてダイアログ

- ツールバーに「アプリについて」ボタンを追加。アプリバージョン、著作権、アイコンライセンス（Flaticon）の帰属リンクを表示。外部リンクは OS のブラウザで開きます

#### マクロエディタ

- マクロステップを**ドラッグ＆ドロップで並べ替え**できます（旧上下ボタンは廃止）。`@dnd-kit/sortable` ベースで、キーボード操作にも対応

#### ウィンドウ状態

- ウィンドウ位置・サイズ・最大化状態を `tauri-plugin-window-state` で保存し、次回起動時に復元します

#### 置換変数

- **ロケール依存日時変数**（OS のロケールに従う）:
  - `\a` 曜日省略名（例 `月` / `Mon`）
  - `\A` 曜日完全名（例 `月曜日` / `Monday`）
  - `\b` 月省略名（例 `5月` / `May`）
  - `\B` 月完全名（例 `5月` / `May`）
  - `\p` AM/PM 表記（例 `午前` / `AM`）
- **大文字小文字制御変数**（キャプチャや他の変数にも作用します）:
  - `\u` 直後 1 文字を大文字
  - `\l` 直後 1 文字を小文字
  - `\U` ... `\E` 範囲を大文字
  - `\L` ... `\E` 範囲を小文字

### UX 変更

- 下部バーの「クリア」ボタンを「再読み込み」に改名。再読み込み後も行選択が保持されるようになりました（旧仕様では毎回選択解除されており、混乱の元になっていました）

### 内部

- ロケール変数・case 制御変数のテストを 8 件追加（計 84 件 PASS）
- 新規依存: `tauri-plugin-window-state`, `tauri-plugin-opener`, `sys-locale`

### 動作環境

| | Windows | macOS |
| --- | --- | --- |
| OS | Windows 10 以降 | macOS 10.15 (Catalina) 以降 |
| アーキテクチャ | x86_64 | x86_64、Apple Silicon (ARM) |

### 既知の制限

- Windows の未マップネットワークホスト（`\\server\share` のサーバ一覧をプログラム的に列挙する機能）は未実装。UNC パスを直接指定する場合は「場所を選択…」ダイアログで入力してください
