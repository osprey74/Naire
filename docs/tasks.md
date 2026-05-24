# Naire — タスク管理

> HANDOFF_naire.md の「実装優先順位」を 1 フェーズ 1 セクションに展開して管理する。
> 完了マーク: `[x]`

## 進捗サマリー

| フェーズ | 概要 | 状態 |
|---|---|---|
| 1 | 骨格（FolderTree + ファイル一覧 + PreviewPanel） | 完了 |
| 2 | 表示フィルタ（list_entries で絞り込み） | 完了 |
| 3 | 対象選択処理（プレビュー行選択） | 完了 |
| 4 | 正規表現モード | 完了 |
| 5 | サポートボタン（regex から） | 完了 |
| 6 | 連番カウンタ（10 進 + 16 進 + 英大文字） | 完了 |
| 7 | UNDO 機構 | 完了 |
| 8 | 定型実装（22 variants） | 完了 |
| 9 | ワイルドカードモード | 完了 |
| 10 | 文字変換モード | 完了 |
| 11 | マクロシステム | 完了 |
| 12 | マクロ JSON 入出力 | 完了 |
| 13 | 設定永続化 | 完了 |
| 14 | フォルダ集約（共通プレフィックス + 移動 + 連番リネーム） | 完了 |
| Post-v1.0.0 | フォルダツリーペイン強化（v1.1.0） | 完了 |

完了フェーズ: 14 / 14 🎉  +  v1.1.0 QoL リリース

---

## Phase 1 — 骨格

- [x] バックエンド: `preview_rename` を空 steps でフォルダ列挙のみ返すよう実装
- [x] フック: `usePreview`
- [x] コンポーネント: `Toolbar`（ターゲット / 再帰 / 深さ / フィルタ入力）
- [x] コンポーネント: `FolderTree`（フォルダ選択 + 簡易ナビゲーション）
- [x] コンポーネント: `PreviewPanel`（3 カラムテーブル）
- [x] コンポーネント: `ActionBar`（ボタン枠のみ）
- [x] コンポーネント: `ModePanel`（タブ切り替え枠）
- [x] コンポーネント: `SequencePanel`（連番設定 UI）
- [x] App.tsx で全体レイアウトを構成

> 備考: Toolbar のフィルタ入力欄は接続済みで、`*`・`*.txt;*.md` などの glob・`^(...)` 否定が動作する（バックエンド `DisplayFilter` を `preview_rename` で利用）。Phase 2 ではプリセット + 履歴 UI を追加する。

## Phase 2 — 表示フィルタ

- [x] バックエンド: `DisplayFilter` を `preview_rename` に組み込み（Phase 1 で完了）
- [x] `FilterCombo` コンポーネント: 入力 + ドロップダウン（プリセット + 履歴）
- [x] プリセット 7 種類（すべて / 画像 / 動画 / 音楽 / ドキュメント / アーカイブ / 隠しファイル除外）
- [x] 履歴: Enter / プリセット選択時に追加、重複削除して最前面、最大 10 件
- [x] キーボード: Enter=コミット、Esc=クローズ、↓=ドロップダウンを開く

> 備考: 履歴の永続化は Phase 13（設定永続化）で行う。現在はセッション内のみ保持。

## Phase 3 — 対象選択処理

- [x] App.tsx に `selectedPaths: Set<string>` state を追加
- [x] PreviewPanel: 行クリックで単一選択、Ctrl+クリックでトグル、Shift+クリックで範囲選択
- [x] テーブル外の空白クリックで選択解除
- [x] ステータスバー: 選択中件数表示 + 「すべて選択」「選択解除」リンク
- [x] パス間引き: items 更新時に存在しないパスを selectedPaths から自動的に除外
- [x] CSS: 選択行のハイライト（青系）、変更行 × 選択行の二重ハイライト

> 備考: バックエンドへの `selectedIndexes` 引き渡しは Phase 4（正規表現モード）で実施する。Phase 3 はステップ未実装のため、選択は UI 状態のみで完結し再フェッチを発生させない。
> アンカー（直近クリック位置）は PreviewPanel のローカル ref で保持。

## Phase 4 — 正規表現モード

### バックエンド
- [x] `rename/variables.rs`: `FileContext` + `expand_template` 実装
  - `\0 \t \e \f \F` の解決
  - `\1`〜`\9` キャプチャグループ
  - `\\` リテラル
  - 未対応エスケープ（`\Y` 等）はリテラル出力（Phase 6 以降で展開予定）
- [x] `rename/advanced.rs`: `apply_regex` で正規表現リネームを実装
- [x] `commands.rs`:
  - `CompiledStep::Regex` + `compile_steps` で事前にコンパイル
  - `build_preview` 共通ヘルパー（preview と execute で共有）
  - `preview_rename` で `is_changed = renamed != original` を埋める
  - `execute_rename` 実装: バッチ内重複・チェーン・既存ファイル衝突を検証してから `std::fs::rename`
  - `selectedIndexes` 部分選択時は選択行のみにステップ適用、非選択行は `renamed = original`
- [x] テスト 14 件 PASS（FileContext / template / 正規表現適用 / 基本変換）

### フロントエンド
- [x] `ModePanel/advanced/RegexMode.tsx`: 検索・置換の 2 行入力 + 変数ヒント
- [x] `ModePanel.tsx`: advancedTab 状態を上位に持ち上げ、RegexMode をレンダ
- [x] `App.tsx`:
  - `regexSearch` / `regexReplace` の状態管理
  - `steps` を mode/advancedTab/search 条件で組み立て
  - `selectedPaths` → `selectedIndexes` を `lastItems` 経由で安定派生
  - `execute_rename` 呼び出しと結果ハンドリング
  - 通知バナー（成功・エラー）
- [x] `ActionBar`: `canRename` を items の変更検出で活性化

> 備考:
> - 連番 `?` / 日時 `\Y` などはまだリテラル扱い。Phase 6 で連番、後続フェーズで日時を実装する。
> - 大文字小文字制御 `\u \U \l \L \E` も未実装。
> - チェーン（a→b と b→c が同一バッチに含まれる）が検出されたら明示エラーで停止。temp 名経由の解決は Phase 7（UNDO）以降で検討。

## Phase 5 — サポートボタン

- [x] `SupportButton/insert-at-cursor.ts`: カーソル位置に文字列を挿入（選択範囲があれば置換）。フォーカス未取得時は末尾追記
- [x] `SupportButton/support-items.ts`: 6 コンテキスト分の menu 定義
  - `regex.search`: 標準 / 文字クラス / メタ文字 / グループの 22 項目
  - `regex.replace` (= `wildcard.replace`): ファイル / 連番 / 日時 / キャプチャ / 大文字小文字制御の 39 項目
  - `wildcard.search`: 2 項目（Phase 9 で使用予定）
  - `char.search` / `char.replace`: 空（Phase 10 で実装）
- [x] `SupportButton/SupportButton.tsx`: ドロップダウン UI、外クリックで閉じる、`requestAnimationFrame` 経由でフォーカス＆カーソル位置を復元
- [x] `RegexMode.tsx` の検索・置換フィールド右肩に組み込み
- [x] `\orig` はマクロ専用 — Phase 11 で `regex.replace` のマクロ版コンテキストに追加予定

> 備考: 連番（`?`〜`????`）と日時系（`\Y \m \d` 等）の挿入候補は提供しているが、バックエンドの展開実装は Phase 6（連番） と後続フェーズで行う。それまで Naire 上では未対応エスケープはリテラル出力。

## Phase 6 — 連番カウンタ

### バックエンド
- [x] `sequence.rs`: `format_alpha` を **bijective base-26**（0-indexed）で実装
  - `digits` パラメータは alpha 時は無視（パディングしない）
  - 値 0 → `A`、値 26 → `AA`、値 701 → `ZZ`、値 702 → `AAA`
  - 当初は positional + min-width padding で実装したが、値 0 と値 26 が両方 "AA" になる重複や、値 676 で突然 3 文字に増える挙動の違和感があり bijective に変更（HANDOFF マッピング表とも整合）
- [x] `sequence.rs` テスト: bijective 単/2/3 文字、`digits` 無視確認、`parse_numbering` 計 6 件
- [x] `variables.rs` `FileContext`: `seq_value: u64`, `seq_numbering: Numbering` フィールド追加
- [x] `variables.rs` `expand_template`:
  - 連続する `?` を最大 4 個まで連番変数として展開（`?`〜`????`）
  - `\?` は連番ではなくリテラル `?`（エスケープハッチ）
  - `format_seq(ctx.seq_value, digits, ctx.seq_numbering)` で書式化
- [x] `commands.rs` `build_preview`:
  - `SequenceConfigDto` を引数追加
  - ステップ適用対象行のみカウンタを進める（非選択行は消費しない）
  - `reset_per_folder=true` 時、`item.folder` が前回と異なれば `counter = seq.start`
  - `counter = counter.saturating_add(step)` でオーバーフロー安全に進める
- [x] `preview_rename` / `execute_rename` の `_seq` を `seq` に変更して `build_preview` へ転送
- [x] テスト計 22 件 PASS（既存 14 + 新規 8）

### フロントエンド
- 変更なし: `SequencePanel` と `usePreview` の SequenceConfig 引き渡しは Phase 1 で既に通電済み

> 注:
> - 日時系変数（`\Y \m \d \H` 等）と大文字小文字制御（`\u \U \l \L \E`）はまだリテラル出力。後続フェーズで実装

## Phase 7 — UNDO 機構

### バックエンド
- [x] `rename/undo.rs::undo_ops(&[OpView])`:
  - 事前検証: 各 op の `new_path` 存在 / `old_path` 空き
  - 逆順実行: 最新のリネームから順に `new_path → old_path`
  - 途中失敗時は部分実行状態でエラー（呼び出し側がスタックに record を残して再試行できる）
- [x] `commands.rs::undo_rename` は `undo_ops` への薄ラッパー
- [x] tempdir ベースのテスト 3 件追加: 正常 / new_path 不在 / old_path 占有 → 計 25 件 PASS

### フロントエンド
- [x] `App.tsx::undoStack: RenameRecord[]` — 最大 20 件、超過分は古い順に削除（`slice(-20)`）
- [x] `execute_rename` 成功時に push
- [x] `onUndo` ハンドラ: `invoke("undo_rename", { record })` → 成功で pop、失敗ならスタックに残す
- [x] Ctrl+Z / Cmd+Z キーボードショートカット
  - INPUT/TEXTAREA フォーカス中はネイティブ編集 UNDO を尊重して傍受しない
  - Shift+Z / Alt+Z は無視（REDO は実装しないため）
  - `onUndoRef` 経由で最新ハンドラを参照（useEffect 依存配列を空に保つ）
- [x] `ActionBar`: `undoCount` を表示、`Ctrl+Z` バッジを `<span class="kbd">` で装飾

> 仕様補足:
> - REDO は実装しない（HANDOFF 仕様）
> - スタックは永続化しない（アプリ終了時にクリア）
> - 1 record = 1 バッチ操作（中身に何千ファイル含んでも 1 件）

## Phase 8 — 定型実装

### バックエンド
- [x] `FileContext` 拡張: `mtime: DateTime<Local>`, `size: u64`, `applied_index: u64`
- [x] `expand_template` リファクタ: `caps: Option<&Captures>` 化（マクロ・定型でも再利用可能）
- [x] 日時変数 `\Y \y \m \d \H \I \M \S` 実装、`\#X` 先行ゼロ除去修飾子も対応
- [x] ファイルサイズ変数 `\;`（3 桁区切り）/ `\:`（区切りなし）
- [x] `rename/convert.rs`: `hankaku_kata_to_zenkaku` / `zenkaku_kata_to_hankaku` 追加（NFC/NFD で濁点・半濁点を合成・分解）
- [x] `rename/builtin.rs`: 22 variant の `BuiltinOp` enum + `apply_builtin` ディスパッチ
  - 連番・追加: `add_seq_str` / `add_datetime` / `add_folder_name` / `add_folder_seq` / `truncate_from_start` / `truncate_from_end`（6 variants）
  - 削除: `delete_chars` / `delete_before` / `delete_pattern`（9 サブパターン）/ `make_83`（4 variants）
  - 文字変換: `case_convert` / `kana_convert` / `width_convert` / `clear_diacritics` / `remove_voiced_mark`（5 variants）
  - 置換: `string_replace`
  - 数値: `number_pad` / `number_adjust`（n 番目の数値を検出して整形・増減）
  - 拡張子: `ext_convert` / `ext_delete` / `ext_add` / `ext_replace`（4 variants）
- [x] `commands.rs::compile_steps` で `"builtin"` kind を受け付け、`op` フィールドを `BuiltinOp` にデシリアライズ
- [x] `build_preview` で `mtime` / `size` をファイル metadata から取得、`applied_index` を選択行で進める
- [x] テスト計 52 件 PASS（builtin 18 + variables 14 + 既存 20）

### フロントエンド
- [x] `ModePanel/builtin/builtin-defaults.ts`: 22 variant のカテゴリ分類と `initialOp(kind)` 初期値ジェネレータ
- [x] `BuiltinMenu.tsx`: カテゴリ折りたたみ可能なリスト、選択ハイライト
- [x] `BuiltinParams.tsx`: 各 variant 専用のパラメータフォームを `switch` で分岐
- [x] `Field` ヘルパー: `useId` + `htmlFor` + `cloneElement` で a11y 対応
- [x] `App.tsx`: `builtinOp` state、`steps` 配列への組み込み、`ModePanel` への配線

### 仕様との差異
- HANDOFF の「46 templates」UI ではなく **22 variants** で実装（カテゴリ表示）
  - 各 variant のパラメータをフォーム入力で自由に変えられる
  - FR スタイルの 46 行プリセット UI は将来のリファイン候補
- `add_seq_str` は stem を `{prefix}{seq}{suffix}` で**置換**（拡張子は維持）と解釈
  - HANDOFF テーブル #5 #6（連番を先頭/末尾に追加）は `prefix` / `suffix` の使い分けで再現
- ロケール依存変数 `\a \A \b \B \p` は Phase 8 では未実装（リテラル出力）
- 大文字小文字制御 `\u \U \l \L \E` も未実装（リテラル出力）

> 注: `add_seq_str` / `add_folder_seq` の連番は op の `start` / `step` / `digits` を使う（HANDOFF 仕様）。`numbering` と `reset_per_folder` は `SequenceConfig` のグローバル値を共有。`build_preview` で `applied_index` をフォルダ遷移時にリセットして実現。

## Phase 9 — ワイルドカードモード

### バックエンド
- [x] `rename/advanced.rs::wildcard_to_regex(pattern) -> String`
  - `?` → `.`、`*` → `.*`、`.` `\\` `(` `)` `[` `]` `{` `}` `+` `^` `$` `|` → エスケープ
  - パターン全体を `^...$` でアンカー（ファイル名全体マッチ）
  - キャプチャ無し（`\1`〜`\9` 不可、代わりに `\0` `\t` `\e` 等を使用）
- [x] `commands.rs::compile_steps` に `"wildcard"` kind 追加: 変換後の正規表現を既存の `CompiledStep::Regex` で処理
- [x] テスト 3 件追加（基本変換 / メタ文字エスケープ / アンカー検証）→ 計 55 件 PASS

### フロントエンド
- [x] `ModePanel/advanced/WildcardMode.tsx`: RegexMode と同じレイアウトで `SupportButton` のコンテキストを `wildcard.search` / `wildcard.replace` に差し替え
- [x] `ModePanel.tsx` で `advancedTab === "wildcard"` 時に WildcardMode をレンダ
- [x] `App.tsx`: `wildcardSearch` / `wildcardReplace` state、`steps` 配列への組み込み
- [x] SupportButton: `wildcard.search` メニューは Phase 5 で既に定義済み（`?` `*` の 2 項目）。`wildcard.replace` は `regex.replace` と共通

> 備考: `support-items.ts` で wildcard.replace は regex.replace と同じ menu を共有しているため、ユーザは regex モードと同じ感覚で置換変数を挿入できる。ただし、ワイルドカードはキャプチャを生成しないため `\1`〜`\9` は無効（空文字列に展開される）。

## Phase 10 — 文字変換モード

### バックエンド
- [x] `rename/char_convert.rs`:
  - `parse_charset(s)`: 文字列を要素列にパース
    - カンマ含む → カンマ区切り列挙（マルチ文字トークン対応）
    - カンマ無し → `X-Y` 範囲 + 残りは 1 文字 1 要素
  - `build_mapping(from, to)`: `(from_token, to_token)` 配列を生成
    - `to` が短いと末尾要素でパディング、空なら削除（`tr` 風）
    - **長いトークン優先のソート**で貪欲マッチを保証
  - `apply(input, mapping)`: 入力を貪欲走査して置換
- [x] `commands.rs::compile_steps` に `"char_convert"` kind 追加
- [x] テスト 11 件追加（範囲 / 列挙 / 全角数字 / ローマ数字貪欲 / 削除 / パディング 等）→ 計 66 件 PASS

### フロントエンド
- [x] `SupportButton/support-items.ts`: `char.search` / `char.replace` の `CHAR_PRESETS` を埋める
  - 英字 (4 種) / 仮名 (2 種) / 数字 (4 種、漢数字含む) / ローマ数字 (大小 2 種)
- [x] `ModePanel/advanced/CharConvertMode.tsx`: from/to 入力 + SupportButton + ヒント
- [x] `ModePanel.tsx` で `advancedTab === "char_convert"` 時に CharConvertMode をレンダ
- [x] `App.tsx`: `charFrom` / `charTo` state、`steps` 配列への組み込み

> 仕様メモ:
> - HANDOFF の TS 型 `{ kind: "char_convert"; from: string; to: string }` に準拠
> - 範囲記法と列挙の混在は不可（カンマがあれば列挙、無ければ範囲。シンプルな分岐）
> - 後続フェーズ用途: 漢数字 → アラビア数字、ローマ数字 → アラビア数字、全角 → 半角 など

## Phase 11 — マクロシステム

### バックエンド
- [x] `FileContext` リファクタ: `stem` / `extension` フィールドを削除し、`expand_template` で `current` 引数から派生する形へ
- [x] `expand_template(template, caps, ctx, current)`: シグネチャに `current: &str` を追加
  - `\0 \t \e` は **`current` から派生**（マクロ各ステップの直前出力を参照）
  - `\orig` (5 文字消費) を実装: マクロ開始時点の元名 = `ctx.original_full`
  - 非マクロ単一ステップでは `current == ctx.original_full` のため `\0` と `\orig` は同じ結果
- [x] `commands.rs::init_macro_items`: 列挙して `StepItem` 群を返す（`original_name == current_name`、metadata から size/mtime）
- [x] `commands.rs::apply_macro_step`: 単一ステップを items 全体に適用
  - 部分選択時は選択行のみ適用、非選択行は `current_name` 維持
  - `FileContext.original_full = item.original_name` で `\orig` を step 0 名にバインド
  - `current = item.current_name` で `\0 \t \e` を直前ステップ出力に対応付け
  - 連番カウンタとフォルダリセットの挙動は `build_preview` と同じ
- [x] `commands.rs::apply_rename_to_filesystem`: 確定済み (path, new_name) をディスクへ反映
  - `execute_rename` と同じ検証（バッチ内重複・チェーン・既存衝突）
  - `RenameRecord` を返却して UNDO スタックに積める
- [x] テスト: `\orig` マクロ変数の動作確認テストを追加 → 計 68 件 PASS

### フロントエンド
- [x] `ModePanel/macro/step-defaults.ts`: `initialStep(kind)` / `describeStep(step)`
- [x] `ModePanel/macro/StepRow.tsx`: 1 ステップ編集行
  - kind selector（定型 / 正規表現 / ワイルドカード / 文字変換）
  - 種別別フォーム（regex/wildcard/char_convert はインラインで実装、builtin は既存の `BuiltinMenu` + `BuiltinParams` を再利用）
  - 削除ボタン + 上下移動ボタン
- [x] `ModePanel/macro/MacroEditor.tsx`: モーダル
  - マクロ名入力 + ステップリスト + 追加ボタン (4 種類)
  - 保存 / キャンセル / 削除
  - 背景クリックで閉じる
- [x] `ModePanel/macro/MacroPanel.tsx`: マクロモードのメイン UI
  - マクロ選択ドロップダウン（進行中はロック）
  - [新規] [編集] ボタン
  - ステップ進行表示: N / 全体
  - 次のステップ概要表示
  - [◀戻す] [ステップ処理▶] [すべて適用]
- [x] `App.tsx`:
  - `macros: Macro[]` / `currentMacroId` / `editingMacro` / `macroItems: StepItem[]` / `macroStepIndex`
  - `initMacroItems` / `applyOneStep` ヘルパー
  - `onMacroStepForward` / `onMacroApplyAll` / `onMacroReset` / `onMacroCreateNew` / `onMacroEdit` ハンドラ
  - フォルダ/フィルタ/モード変更時にマクロ実行状態を自動リセット
  - マクロモード中の `onRename` は `apply_rename_to_filesystem` を呼ぶ
  - マクロモード中の PreviewPanel は `macroItems` を `PreviewItem` に変換して表示
  - 成功時に UNDO スタックへ push（既存と統合）

### 仕様メモ
- `\orig` は `\o` + `rig` の 5 文字エスケープ。expand_template で先頭一致でチェック
- マクロ進行中（step_index > 0）はマクロ選択ドロップダウンをロック
- [すべて適用] は残りステップを順次 invoke。失敗したらそこで停止
- マクロステップの reorder は dnd-kit ではなく **上下ボタン**で実装（シンプル化）。dnd-kit への移行は将来検討
- **永続化はまだしていない**（Phase 13 で `tauri-plugin-store` に保存）。アプリ再起動でマクロは消える

## Phase 12 — マクロ JSON インポート／エクスポート

### バックエンド
- [x] `macro_io.rs`:
  - `parse_macros_json(text)`: 単一オブジェクト・配列の自動判別、`id` 再生成
  - `serialize_macros(macros)`: 1 件はオブジェクト、複数は配列で出力
  - `sanitize_filename(name)`: Windows/macOS 禁止文字を `_` に置換、空文字は `macro` にフォールバック
  - `import_via_dialog(app)`: tauri-plugin-dialog でファイル選択 → 読み込み → パース
  - `export_via_dialog(app, macros)`: デフォルトファイル名生成（単一 = `naire_macro_{name}.json`、複数 = `naire_macros.json`）→ 保存ダイアログ → 書き込み
- [x] `commands.rs`: `export_macros` / `import_macros` を `macro_io` の関数に薄ラッパー
- [x] `RenameStepDto` に `skip_serializing_if = "Option::is_none"` を追加 → JSON 出力が `kind` 不要なフィールドを含まずクリーンに
- [x] テスト 8 件追加（単一/配列のパース、空白付きパース、エラー処理、シリアライズ形式、ファイル名 sanitize）→ 計 76 件 PASS

### フロントエンド
- [x] `MacroPanel.tsx`: ↑インポート / ↓全エクスポート ボタンを追加（マクロ選択行の下に IO 行を新設）
- [x] `MacroEditor.tsx`: フッターに ↓エクスポート ボタンを追加（編集中マクロの単体保存）。`name.trim()` を反映して保存前の名前で出力
- [x] `App.tsx`: `onMacroImport` / `onMacroExportAll` / `onMacroEditorExport` ハンドラ
  - インポート成功時は `macros` 配列に push、件数を通知
  - キャンセル時はエラー通知を出さない（メッセージに「キャンセル」を含むかで判定）

### Claude 連携用フォーマット
HANDOFF 仕様の単一マクロ JSON 例:
```json
{
  "id": "uuid-v4",
  "name": "作者ソートキー追加",
  "created_at": "2026-05-10T20:00:00+09:00",
  "updated_at": "2026-05-10T20:00:00+09:00",
  "steps": [
    {"kind": "regex", "search": "^\\[(.)(.*?)\\].*", "replace": "\\l\\1"},
    {"kind": "builtin", "op": {"type": "kana_convert", "conversion": "kata_to_hira", "skip_ext": false}},
    {"kind": "builtin", "op": {"type": "remove_voiced_mark", "skip_ext": false}},
    {"kind": "regex", "search": "^(.*)$", "replace": "\\1 \\orig"}
  ]
}
```
`id` はインポート時に再生成されるので任意の文字列で OK。Claude で生成して直接 Naire に取り込めるフォーマット。

> 仕様メモ:
> - キャンセル時の判定はメッセージ内容（「キャンセルされました」）でやっており、I18N 対応時に見直し必要
> - 既存のマクロと同名でもインポートできる（id が違うので別物扱い）

## Phase 13 — 設定永続化

### フロントエンド
- [x] `hooks/useConfig.ts`:
  - `useLoadConfig()`: アプリ起動時に `Store.load('config.json')` で 1 回だけ読み込み、`{ loaded, initialConfig }` を返す
  - `persistConfig(cfg)`: ストアへ書き込み + `save()` でディスク永続化
  - Store インスタンスはモジュール内で 1 つに固定（再 load 防止）
- [x] `App.tsx`:
  - `useLoadConfig` の結果から起動時に各 state を復元（`configApplied` フラグで上書き保存を防止）
  - 関連 state が変わるたびに 500ms debounce で `persistConfig` 呼び出し
  - 永続化対象: `last_folder` / `last_mode` / `last_target` / `recursive` / `depth` / `filter` / `filter_history` / `seq` / `macros`

### 永続化対象（AppConfig）
| フィールド | 内容 | 反映タイミング |
|---|---|---|
| `last_folder` | 直近のフォルダパス | フォルダ変更 |
| `last_mode` | 直近のモード（builtin / advanced / macro） | モード切替 |
| `last_target` | ファイル / フォルダ | ターゲット切替 |
| `recursive` / `depth` | 再帰設定 | 変更時 |
| `filter` / `filter_history` | フィルタ + 履歴（最大 10 件） | フィルタ確定時 |
| `seq` | 連番カウンタ設定 | 変更時 |
| `macros` | マクロ一覧 | 保存・削除・インポート時 |

### 永続化しないもの（HANDOFF 仕様 or 設計判断）
- UNDO スタック（HANDOFF 仕様）
- 部分選択状態（transient UI）
- regex/wildcard/char_convert/builtin の入力中の値（transient UI）
- 編集中マクロ（保存時に macros 配列へ反映されるのでそこで永続化）
- マクロ実行状態（macroItems / macroStepIndex — transient）

### 既知の制限
- **ウィンドウ状態（サイズ・位置）は placeholder のみ保存**。restore 処理は未実装。`tauri-plugin-window-state` を導入するか、Tauri Window API で個別実装するかは将来検討
- 設定読み込み失敗時はサイレントにデフォルトへフォールバック（コンソールエラーのみ）
- 設定ファイルパス: Tauri が標準で割り振る `appData` 配下に `config.json`

## Post-v0.1.0 — UI ポリッシュ

- [x] アプリについてダイアログ（バージョン + Flaticon 帰属リンク、`tauri-plugin-opener` で外部 URL を OS ブラウザに）
- [x] エクスプローラー風フォルダツリー（`list_folder_tree` コマンド + 再帰 TreeNode + auto-expand）。macOS は `$HOME` ルート、Windows は接続中ドライブ列挙
- [x] ActionBar の「クリア」を「再読み込み」へ（選択は path ベースで保持）

## Post-v0.1.0 — 既知の制限事項対応

- [x] **ウィンドウサイズ・位置の復元** — `tauri-plugin-window-state` 導入。`AppConfig.window` プレースホルダは削除（プラグインが独立した state ファイルで管理）
- [x] **マクロステップの D&D 並べ替え** — `@dnd-kit/sortable` で StepRow を sortable 化（上下ボタンは drag handle に置換）。`useLayoutEffect` で transform を imperative 適用してインライン style を回避
- [x] **ロケール依存日時変数**（`\a \A \b \B \p`）と **大文字小文字制御変数**（`\u \U \l \L \E`）— `chrono::format_localized` + `sys-locale` で OS ロケール解決、`CaseAcc` アキュムレータで case 修飾子を実装。MACRO_REFERENCE EN/JA も更新。テスト 8 件追加（計 84 件 PASS）
- [x] **ネットワークドライブのブラウズ対応** — macOS: `/Volumes/*`（外部/SMB/AFP マウント）をツリールートに追加。Windows: マップ済みネットワークドライブは既存のドライブ列挙でカバー、未マップ UNC は「場所を選択…」ダイアログで指定可能。プログラム的な Windows ネットワーク列挙（`WNetEnumResource`）は依存追加が重く UX 価値が低いため見送り

## Phase 14 — フォルダ集約

> Naire の「リネームのみ」原則の唯一の例外。複数フォルダを共通プレフィックスから生成した親フォルダに集約し、内部を連番リネームする。
> 詳細仕様は HANDOFF_naire.md の「## フォルダ集約仕様」を参照。

### 14.1 — 共通プレフィックス抽出（バックエンド）

- [x] `src-tauri/src/rename/group.rs` 新設、`rename/mod.rs` に登録
- [x] `longest_common_prefix(names: &[&str]) -> String` 実装
  - char 単位で共通プレフィックス算出
  - 共通部分が入力の完全 prefix なら末尾スペースのみ trim
  - 途中で切れている場合は最後の空白で切り戻し
  - 空白が一切なければ raw prefix を fallback として返す
- [x] テスト 15 件追加（全 99 件 PASS）
  - 日本語混在 / 半角全角スペース trim / 単一要素 / 共通要素なし / 完全一致
  - 完全 prefix / 空白なし fallback / 絵文字 char 境界安全性
  - 提示実例（コミック第N巻パターン）

> 仕様改良ポイント: 当初の「末尾スペースのみ trim」案では `[咲野...]攻略 第0` が残ってしまうため、「最後の空白で切り戻し」ロジックを追加。これにより `第0` のような部分トークンが除去される。空白が一切ないケース（例: `Doraemon-Vol01`/`Doraemon-Vol02`）では raw prefix を fallback として返し、ユーザが UI で編集できる前提とする。

### 14.2 — RenameOp の enum 化 + UNDO 拡張

- [x] `commands.rs` の `RenameOp` を `enum { Rename, CreateDir }` に変更（`#[serde(tag = "type", rename_all = "snake_case")]`）
- [x] 既存のリネーム系コマンド（`execute_rename` / `apply_rename_to_filesystem`）の `RenameRecord` 生成箇所を `Rename` バリアントに更新
- [x] `rename/undo.rs::OpView` を enum 化（`Rename` / `CreateDir`）、`undo_ops` を `CreateDir` 対応に拡張（`std::fs::remove_dir`、空でなければエラー）
- [x] `undo_rename` コマンドの `RenameOp` → `OpView` マッピングを `match` で更新
- [x] TypeScript 型定義 (`src/types/rename.ts`) を discriminated union に更新
- [x] UNDO テスト 4 件追加（**全 103 件 PASS**）:
  - `undo_create_dir_removes_empty_dir`: 空フォルダの削除
  - `undo_create_dir_fails_when_not_empty`: 中身があると失敗
  - `undo_create_dir_fails_when_path_missing`: 存在しないパスは失敗
  - `undo_mixed_record_reverses_in_reverse_order`: 集約操作（CreateDir + Rename×2）の逆順実行
- [x] TypeScript の型チェック (`npx tsc --noEmit`) PASS

### 14.3 — execute_group / compute_group_preview コマンド

- [x] `group.rs` にプラン生成ロジック追加（`GroupRenameSpec` / `GroupItemPlan` / `GroupPlan` / `format_group_rename` / `build_plan` / `find_renamed_collision`）。9 件の単体テスト追加
- [x] `commands.rs` に `GroupRenameDto` / `GroupItemPreview` / `GroupPreview` DTO を追加
- [x] `compute_group_preview` 実装: バリデーション + 共通プレフィックス算出 + 連番リネーム結果のドライラン
  - エラーは `GroupPreview.error` に格納（Tauri Err は使わない）→ UI が常に preview を受け取れる
  - `conflict` フィールドで集約フォルダ名の衝突を soft error として通知
- [x] `execute_group` 実装:
  - `compute_group_preview_impl` を再利用してバリデーション
  - **move + rename を単一の `std::fs::rename` に統合**（最終パスへ直接移動、ops 数を最小化）
  - `RenameRecord = CreateDir + Rename×N` を返却
- [x] 連番リネーム: 定型 #1 の **アルゴリズム** を `format_group_rename` で再実装（`add_seq_str` 関数を直接呼ばないのは、フォルダ名の `.` を拡張子と誤認しないため）
- [x] `lib.rs` の `invoke_handler` に `compute_group_preview` / `execute_group` 登録
- [x] テスト 15 件追加（**全 127 件 PASS**）:
  - **preview 系 (10 件)**: 正常 / prefix・suffix 指定 / 空選択 / 単独選択 / 不在フォルダ / ファイル選択 / 衝突 / step=0 / 空 group_name / rename 省略
  - **execute 系 (4 件)**: 正常 / 衝突阻止 / 不正選択阻止 / rename 省略時の移動のみ
  - **round trip (1 件)**: 集約→UNDO で完全復元
- [x] 同期 `_impl` 関数を分離（テスト容易性 + tokio 不要）

> 設計判断: 当初計画では「移動 → 連番リネーム」の 2 フェーズで 2N ops を想定していたが、最終パスへ直接 `std::fs::rename` する単一フェーズに変更（N ops + CreateDir）。UNDO の挙動も同等に保たれる。

### 14.4 — フロントエンド「集約」タブ

- [x] TypeScript 型定義追加: `GroupRenameDto` / `GroupItemPreview` / `GroupPreview` (src/types/rename.ts)
- [x] `AppConfig.last_mode` に `"group"` バリアントを追加
- [x] `ModePanel.tsx` の `Mode` 型に `"group"` を追加、新タブと `<GroupPanel>` レンダー分岐
- [x] `ModePanel/group/GroupPanel.tsx` 新設:
  - target=file 時 / 選択<2 件時の案内表示
  - 集約フォルダ名入力（自動算出 + 編集可能 + 「自動算出に戻す」ボタン、`nameDirty` フラグで上書き抑止）
  - 「連番リネームを実行する」チェックボックス
  - プレフィックス / サフィックス / 桁数 / 開始 / ステップ 入力（無効時は disabled）
  - エラー（赤）/ 衝突警告（オレンジ）バナー
  - 「集約実行」ボタン
- [x] `GroupPanel.module.css`: 既存パネルと統一感のあるスタイル
- [x] `App.tsx`:
  - `groupState` / `groupPreview` / `groupComputing` state を追加
  - `dirname()` ヘルパで選択フォルダの親パスを導出（Windows / POSIX 両対応）
  - `groupSelection` useMemo: 選択 PreviewItem から親パス・名前・sameParent を派生
  - 200ms debounce で `compute_group_preview` を invoke、結果を反映
  - 自動算出時のみ `groupState.groupName` を共通プレフィックスに同期
  - 異親選択時は preview に上書きエラーを反映（`effectiveGroupPreview`）
  - `onGroupExecute` ハンドラで `execute_group` 呼び出し、成功時に UNDO スタックへ push
  - モード切替 / フォルダ・フィルタ変更時に集約状態をリセット
  - 集約モード中は ActionBar の「リネーム実行」を無効化（独立した「集約実行」を使う）
- [x] **不具合修正**: 集約モード時に PreviewPanel の `items` を集約後パスに切り替えると、PreviewPanel の「items にないパスを自動間引きする」ロジックが選択を即時解除してしまう問題を発見。集約モードでも PreviewPanel は通常のフォルダ一覧を維持し、集約後リネームのプレビューは GroupPanel 内の小テーブルで表示する設計に変更
- [x] TypeScript 型チェック (`npx tsc --noEmit`) / フロントビルド (`npm run build`) / Rust ビルド (`cargo build`) すべて PASS

### 14.5 — 統合テスト + ドキュメント

- [x] E2E 動作確認（実フォルダで集約 → 連番リネーム → UNDO で完全復元）— 総司様の実機確認で完了
- [x] エッジケース: 異親選択 / 既存名衝突 / 空のプレフィックス / 単一選択 — Phase 14.3 で 15 件のテストで網羅
- [x] **`README.md` / `README.ja.md` 新規作成**（v1.0.0 メジャーリリースに合わせて、機能一覧 / インストール / ビルド方法 / アーキテクチャ / 謝辞 を網羅）
- [x] **`RELEASE_NOTES_v1.0.0.md`**（EN+JA）作成、フォルダ集約をハイライト
- [x] バージョン更新: `package.json` / `src-tauri/Cargo.toml` / `src-tauri/tauri.conf.json` を `1.0.0` へ、`Cargo.lock` の naire エントリのみ自動更新

### 仕様メモ

- **v1 制約**: 集約フォルダは選択フォルダと同じ親直下のみ作成。任意の場所への集約は将来検討
- **連番リネーム省略時の動作**: 移動のみ実行、元のフォルダ名を維持
- **prefixDirty フラグ**: ユーザが集約名を手動編集したら自動再計算を抑止
- **UNDO 1 record = 集約 1 回**: CreateDir + 移動×N + 連番リネーム×N を 1 件として扱う
- **クロスプラットフォーム**: `std::fs::rename` と `std::fs::create_dir` / `remove_dir` は Windows / macOS 共通動作

---

## Naire v1.0.0 リリース

**2026-05-17** — General Availability。Phase 14 完了に合わせてメジャーリリースを実施。

- 全 14 フェーズ完了
- バックエンドテスト 127 件 PASS
- README.md / README.ja.md / RELEASE_NOTES_v1.0.0.md を新規作成
- CI/CD タグプッシュで自動ビルド & ドラフトリリース作成

---

## Post-v1.0.0 — フォルダツリーペイン強化

> v1.1.0 として 2026-05-24 リリース。フォルダツリーペインの操作性を集中的に改善。

### バックエンド
- [x] `get_folder_stats(path)` コマンド: 指定フォルダ配下のファイル数と合計バイト数を再帰集計（WalkDir）。権限エラー等のエントリは黙ってスキップ
- [x] `rename_folder(old_path, new_name)` コマンド: 単一フォルダのリネーム。空名・パス区切り文字・既存衝突を弾き、UNDO 可能な `RenameRecord` を返す
- [x] `move_folder_to_trash(path)` コマンド: `trash` クレート v5 で OS 標準のゴミ箱（Windows: Recycle Bin / macOS: Trash）へ移動
- [x] `Cargo.toml` に `trash = "5"` を追加

### フロントエンド — FolderTree
- [x] **ダブルクリックでインラインリネーム**: `.name` を `<input>` に差し替え。Enter で確定、Esc でキャンセル、blur で確定。エラー時は編集モードを維持して再入力可
- [x] **フォルダ内容量の表示**: 各ノードの右側に「N / X MB」を淡色で表示。初回マウント時に `get_folder_stats` を非同期取得し `statsMap` にキャッシュ。ドライブルート（Windows の `C:\` 等）は性能上スキップ
- [x] **右クリックコンテキストメニュー**: `onContextMenu` で `{path, name, x, y}` を state に保持し、メニュー外クリック / Escape で閉じる。「フォルダを削除（ゴミ箱へ移動）」項目から `@tauri-apps/plugin-dialog` の `ask()` で確認 → `move_folder_to_trash` を invoke
- [x] **リネーム / 削除後の整合性確保**: 親フォルダの children を再取得 + childrenMap / expanded / statsMap から旧パス配下を掃除 + 現在選択中フォルダが影響を受ける場合は新パスへ付け替え（リネーム）または親へフォールバック（削除）
- [x] **リロードシグナル**: `reloadSignal: number` prop を追加。値の変化で roots 再取得 + statsMap 破棄 + 展開中フォルダの children 再取得を実行。`expanded` は `expandedRef` 経由で読み deps から除外（展開操作で誤発火させない）

### フロントエンド — App
- [x] **左ペイン幅可変**: `App.module.css` の `grid-template-columns` を `var(--left-width, 220px) 6px 320px 1fr` に変更し、`.resizer` を `<div className={styles.left}>` と `<div className={styles.center}>` の間に挿入。pointer 系イベントを window-level に張るドラッグハンドラ実装、140〜800px に clamp
- [x] **AppConfig.left_width** を追加し永続化フローに統合
- [x] **再読み込みボタン拡張**: ActionBar の `onClear` で `preview.reload()` と `setFolderTreeReload((n) => n + 1)` を併発し、フォルダツリーも再取得
- [x] **`onFolderRenamed`**: リネーム成功時に UNDO スタックへ push + プレビュー再読込
- [x] **`onFolderDeleted` / `onError`**: 既存の `notice` バナーで通知

### ドキュメント
- [x] README.md / README.ja.md の「ワークスペース機能」セクションを更新（インラインリネーム / 内容量 / リサイザ / 右クリック削除 / 再読み込み統合）
- [x] RELEASE_NOTES_v1.1.0.md（EN+JA）作成
- [x] バージョン更新: `package.json` / `src-tauri/Cargo.toml` / `src-tauri/tauri.conf.json` を `1.1.0` へ、`Cargo.lock` を `cargo generate-lockfile` で再生成

### 仕様メモ
- ゴミ箱への移動はアプリ内 UNDO スタックには積まない（OS 側で復元可能なため二重管理回避）
- スタッツの集計はノード可視時にのみトリガー（ドライブルートは除外）。リネーム / 削除 / 再読み込みで該当パス配下のキャッシュを破棄
- リネーム失敗時（権限エラー・既存衝突等）は編集モードを維持し、`renameError` をツリー上部に表示
