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
| 11 | マクロシステム | 未着手 |
| 12 | マクロ JSON 入出力 | 未着手 |
| 13 | 設定永続化 | 未着手 |

完了フェーズ: 10 / 13

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

## Phase 3 以降

未着手。HANDOFF_naire.md の優先順位に従って展開する。
