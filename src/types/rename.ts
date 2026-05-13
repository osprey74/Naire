// ── ターゲット ──────────────────────────────────────────────────
export type TargetType = "file" | "folder";

// ── リネームステップ（Union）───────────────────────────────────
export type RenameStep =
  | { kind: "builtin"; op: BuiltinOp }
  | { kind: "wildcard"; search: string; replace: string }
  | { kind: "regex"; search: string; replace: string }
  | { kind: "char_convert"; from: string; to: string };

// ── 定型操作 ────────────────────────────────────────────
export type BuiltinOp =
  // 連番・文字列の追加
  | { type: "add_seq_str"; prefix: string; suffix: string; digits: number; start: number; step: number }
  | { type: "add_datetime"; format: string; position: "prefix" | "suffix" }
  | { type: "add_folder_name"; position: "prefix" | "suffix" }
  | { type: "add_folder_seq"; digits: number; start: number; step: number }
  | { type: "truncate_from_start"; n: number }
  | { type: "truncate_from_end"; n: number }
  // 数字・文字列の削除
  | { type: "delete_chars"; from: "start" | "end"; offset: number; count: number }
  | { type: "delete_before"; from: "start" | "end"; n: number }
  | { type: "delete_pattern"; pattern: DeletePattern }
  | { type: "make_83" }
  // 文字種の変換
  | { type: "case_convert"; conversion: CaseConversion; skip_ext: boolean }
  | { type: "kana_convert"; conversion: KanaConversion; skip_ext: boolean }
  | { type: "width_convert"; conversion: WidthConversion; skip_ext: boolean }
  | { type: "clear_diacritics"; skip_ext: boolean }
  | { type: "remove_voiced_mark"; skip_ext: boolean }
  // 文字列の置換
  | { type: "string_replace"; search: string; replace: string }
  // 数値の整理
  | { type: "number_pad"; from: "start" | "end"; n: number; digits: number }
  | { type: "number_adjust"; from: "start" | "end"; n: number; delta: number }
  // 拡張子の変換
  | { type: "ext_convert"; conversion: ExtConversion }
  | { type: "ext_delete" }
  | { type: "ext_add"; ext: string }
  | { type: "ext_replace"; ext: string };

export type DeletePattern =
  | "copy_num"
  | "copy_num_vista"
  | "shortcut"
  | "shortcut_vista"
  | "bracket_content"
  | "num_kagi_or_round"
  | "num_kagi"
  | "num_round"
  | "num_lenticular";

export type CaseConversion = "capitalize" | "upper" | "lower";
export type KanaConversion =
  | "hira_to_kata"
  | "kata_to_hira"
  | "hankaku_kata_to_zenkaku"
  | "zenkaku_kata_to_hankaku";
export type WidthConversion = "to_zenkaku" | "to_hankaku";
export type ExtConversion = "upper" | "lower";

// ── マクロ ───────────────────────────────────────────────────
export interface Macro {
  id: string;
  name: string;
  steps: RenameStep[];
  created_at: string;
  updated_at: string;
}

// ── UNDO ─────────────────────────────────────────────────────
export interface RenameOp {
  old_path: string;
  new_path: string;
}
export interface RenameRecord {
  id: string;
  timestamp: string;
  ops: RenameOp[];
}

// ── プレビュー行 ─────────────────────────────────────────────
export interface PreviewItem {
  original: string;
  renamed: string;
  folder: string;
  path: string;
  is_changed: boolean;
}

// ── マクロステップ処理用アイテム ─────────────────────────────
export interface StepItem {
  path: string;
  original_name: string;
  current_name: string;
  folder: string;
  size: number;
  mtime: string;
}

export interface MacroExecState {
  macro_id: string;
  step_index: number;
  items: StepItem[];
}

// ── 連番カウンタ設定（グローバル）────────────────────────────
export interface SequenceConfig {
  start: number;
  step: number;
  reset_per_folder: boolean;
  numbering: "decimal" | "hex" | "alpha";
}

// ── PreviewPanel カラム幅（px）─────────────────────────────────
export interface PreviewColumnWidths {
  original: number;
  renamed: number;
}

// ── アプリ設定 ───────────────────────────────────────────────
export interface AppConfig {
  last_folder: string;
  last_mode: "builtin" | "advanced" | "macro";
  last_target: TargetType;
  recursive: boolean;
  depth: number;
  filter: string;
  filter_history: string[];
  seq: SequenceConfig;
  macros: Macro[];
  column_widths?: PreviewColumnWidths;
}
