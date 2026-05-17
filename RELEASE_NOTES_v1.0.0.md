# Naire v1.0.0 Release Notes

**Release Date:** 2026-05-17

Naire v1.0.0 is the **General Availability** release. It introduces the **folder grouping** feature — the first and only intentional exception to Naire's "rename only" design rule — and marks Naire as a stable, production-ready bulk rename tool.

---

## English

### 🎉 General Availability

Naire reaches v1.0.0 with full feature coverage across every category planned in the HANDOFF specification, plus the new folder grouping workflow. The API surface and on-disk formats (settings store, macro JSON, UNDO record) are now stable.

### Highlights

#### 🆕 Folder grouping (Phase 14)

The headline feature of v1.0.0. Select multiple sibling folders that share a common name prefix and consolidate them into a single new parent folder, optionally renumbering them in one operation.

**Example.** Three folders that share a common book series prefix:

```
parent/
├── [Author] Title Vol01
├── [Author] Title Vol02
└── [Author] Title Vol03
```

After grouping (auto-extracted name, default sequence settings):

```
parent/
└── [Author] Title/
    ├── 01
    ├── 02
    └── 03
```

- **Common-prefix auto-extraction** — char-wise common prefix with smart fallback (truncates to the last whitespace boundary when the prefix cuts mid-token; falls back to the raw prefix otherwise)
- **Editable group name** — the auto-suggested name is editable; an "auto-revert" button restores the suggestion
- **Optional sequence rename** — enable a checkbox to apply `{prefix}{seq}{suffix}` to each grouped folder; sequence numbering inherits the global decimal / hex / alpha (bijective base-26) setting from the Sequence Panel
- **Validation** — same-parent enforcement, conflict detection, rename-duplicate detection, empty-name guard
- **Full UNDO** — the entire group operation (CreateDir + N moves/renames) is one UNDO entry that fully reverses with `Ctrl+Z`

The grouping workflow respects Naire's safety guarantees: all selected folders must be siblings, the target folder cannot be on a different volume, and the operation is gated by a live preview that surfaces errors and conflicts before execution.

### Design surface stability

v1.0.0 commits to backwards-compatible evolution of:

- **`AppConfig`** — settings store schema (`tauri-plugin-store`)
- **Macro JSON format** — single-object and array variants, with auto re-generated UUIDs on import
- **`RenameRecord` / `RenameOp`** — internal UNDO record format (now an enum with `Rename` and `CreateDir` variants; not persisted, regenerated each session)
- **Tauri command surface** — 12 commands covering preview, execute, undo, macro lifecycle, folder enumeration, and folder grouping

### Internal improvements

- **127 backend tests** (up from 84 in v0.2.0) — 43 new tests covering common-prefix extraction, plan generation, the `Rename` / `CreateDir` UNDO ops, and the full grouping command surface including round-trip undo
- `RenameOp` migrated to a tagged enum (`#[serde(tag = "type")]`) to accommodate the new `CreateDir` variant; UNDO continues to enforce single-operation atomicity (no partial commit)
- New module `rename/group.rs` with `longest_common_prefix`, `format_group_rename`, `build_plan`, `find_renamed_collision`
- New Tauri commands `compute_group_preview` and `execute_group`
- New frontend component `ModePanel/group/GroupPanel.tsx` with its own CSS module
- TypeScript types for `GroupRenameDto`, `GroupItemPreview`, `GroupPreview` shared with Rust DTOs

### Documentation

- **`README.md`** / **`README.ja.md`** — new top-level READMEs (English + Japanese), introduced with the v1.0.0 release
- **`HANDOFF_naire.md`** — updated with the formal folder-grouping specification (`## フォルダ集約仕様` section, RenameOp enum extension, new Tauri commands, new module entry in the Rust module tree)
- **`docs/tasks.md`** — Phase 14 broken into 5 sub-phases (14.1 prefix extraction → 14.5 docs), all marked complete

### System Requirements

| | Windows | macOS |
| --- | --- | --- |
| OS | Windows 10 or later | macOS 10.15 (Catalina) or later |
| Architecture | x86_64 | x86_64, Apple Silicon (ARM) |

### Migration from v0.2.0

No breaking changes. Existing settings, macro libraries, and window state carry over unchanged. The UNDO stack does not persist (HANDOFF spec), so the `RenameOp` enum change has no migration impact.

### Known Limitations

- Programmatic Windows network enumeration (browsing `\\server\share` hosts not mapped to a drive letter) is not implemented — use「場所を選択…」to type a UNC path directly.
- Folder grouping is restricted to same-volume operations; cross-volume moves are not supported (would require copy + delete with rollback; deferred to future consideration).

### What's next

v1.0.0 closes the planned feature set. Future releases will focus on UX polish, performance, and community-requested enhancements.

---

## 日本語

### 🎉 正式リリース（GA）

Naire は v1.0.0 で **正式リリース** となります。HANDOFF 仕様で計画していた全機能カテゴリを網羅し、さらに新機能「**フォルダ集約**」を搭載しました。API 表面とディスク上のフォーマット（設定ストア、マクロ JSON、UNDO レコード）はこれ以降後方互換性を保ちます。

### ハイライト

#### 🆕 フォルダ集約（Phase 14）

v1.0.0 の目玉機能でございます。共通プレフィックスを持つ兄弟フォルダを複数選択し、1 操作で新規親フォルダにまとめつつ、オプションで連番リネームも同時に適用できます。

**例**: 同じシリーズ名を持つ 3 フォルダ:

```
parent/
├── [作者名] タイトル 第01巻
├── [作者名] タイトル 第02巻
└── [作者名] タイトル 第03巻
```

集約後（自動算出名・デフォルト連番設定）:

```
parent/
└── [作者名] タイトル/
    ├── 01
    ├── 02
    └── 03
```

- **共通プレフィックスの自動抽出** — char 単位で共通先頭部分を計算、途中で切れている場合は最後の空白で切り戻し、空白がなければ raw prefix を fallback として返す
- **集約フォルダ名は編集可** — 自動算出値を任意に編集可能、「自動算出に戻す」ボタンで元に戻せる
- **連番リネームはオプション** — チェックボックスで ON/OFF、`{prefix}{seq}{suffix}` を各フォルダに適用、進数（10 進 / 16 進 / 英大文字 bijective base-26）は連番設定パネルから継承
- **入念なバリデーション** — 同一親チェック / 衝突検出 / リネーム結果の重複検出 / 空名ガード
- **完全な UNDO 対応** — 集約操作全体（CreateDir + N 件の移動・リネーム）が 1 UNDO エントリにまとめられ、`Ctrl+Z` で完全に巻き戻り

集約ワークフローは Naire の安全保証を遵守します。選択フォルダは全て兄弟である必要があり、集約先は異なるボリュームには作成できず、ライブプレビューで実行前にエラー・衝突を確認できます。

### 設計表面の安定化

v1.0.0 以降は以下について後方互換性を約束します:

- **`AppConfig`** — 設定ストアのスキーマ（`tauri-plugin-store`）
- **マクロ JSON フォーマット** — 単一オブジェクト / 配列の両形式、インポート時に UUID 自動再生成
- **`RenameRecord` / `RenameOp`** — 内部 UNDO レコード形式（Phase 14 で enum 化、`Rename` / `CreateDir` バリアント。永続化はせず毎セッション再生成）
- **Tauri コマンド表面** — プレビュー、実行、UNDO、マクロライフサイクル、フォルダ列挙、フォルダ集約をカバーする 12 コマンド

### 内部改善

- **バックエンドテスト 127 件**（v0.2.0 の 84 件から 43 件追加）— 共通プレフィックス抽出、プラン生成、`Rename` / `CreateDir` UNDO op、集約コマンド全体（UNDO ラウンドトリップ含む）をカバー
- `RenameOp` を tagged enum 化（`#[serde(tag = "type")]`）し新規 `CreateDir` バリアントを追加。UNDO の単一操作アトミック性は維持
- 新規モジュール `rename/group.rs`（`longest_common_prefix` / `format_group_rename` / `build_plan` / `find_renamed_collision`）
- 新規 Tauri コマンド `compute_group_preview` / `execute_group`
- 新規フロントコンポーネント `ModePanel/group/GroupPanel.tsx` と専用 CSS モジュール
- Rust DTO と共有する TypeScript 型: `GroupRenameDto` / `GroupItemPreview` / `GroupPreview`

### ドキュメント整備

- **`README.md`** / **`README.ja.md`** — v1.0.0 リリースに合わせてプロジェクトトップレベルの README を新規作成（英語 + 日本語）
- **`HANDOFF_naire.md`** — フォルダ集約の正式仕様セクション（`## フォルダ集約仕様`）、`RenameOp` enum 拡張、新規 Tauri コマンド、Rust モジュールツリーへの追加
- **`docs/tasks.md`** — Phase 14 を 5 サブフェーズ（14.1 共通プレフィックス → 14.5 ドキュメント）に分解、全て完了

### 動作環境

| | Windows | macOS |
| --- | --- | --- |
| OS | Windows 10 以降 | macOS 10.15 (Catalina) 以降 |
| アーキテクチャ | x86_64 | x86_64、Apple Silicon (ARM) |

### v0.2.0 からの移行

破壊的変更はございません。既存の設定・マクロライブラリ・ウィンドウ状態はそのまま引き継がれます。UNDO スタックは永続化していない（HANDOFF 仕様）ため、`RenameOp` enum 化の影響もありません。

### 既知の制限

- Windows の未マップネットワークホスト（`\\server\share` のサーバ一覧をプログラム的に列挙する機能）は未実装。UNC パスを直接指定する場合は「場所を選択…」ダイアログをご利用ください
- フォルダ集約は同一ボリューム内のみ。異ボリューム移動は未対応（copy + delete + ロールバックが必要なため将来検討）

### 今後

v1.0.0 で計画機能セットを完了といたしました。今後のリリースは UX 改善・パフォーマンス・コミュニティからの要望対応に注力してまいります。
