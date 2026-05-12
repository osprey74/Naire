import type { BuiltinOp } from "../../../types/rename";

export type BuiltinKind = BuiltinOp["type"];

export interface BuiltinEntry {
  kind: BuiltinKind;
  label: string;
}

export const BUILTIN_CATEGORIES: { title: string; items: BuiltinEntry[] }[] = [
  {
    title: "連番・文字列の追加",
    items: [
      { kind: "add_seq_str", label: "文字列＋連番＋文字列" },
      { kind: "add_datetime", label: "日時を追加" },
      { kind: "add_folder_name", label: "フォルダ名を追加" },
      { kind: "add_folder_seq", label: "フォルダ名＋連番" },
      { kind: "truncate_from_start", label: "先頭から n 文字を削除" },
      { kind: "truncate_from_end", label: "末尾から n 文字を削除" },
    ],
  },
  {
    title: "数字・文字列の削除",
    items: [
      { kind: "delete_chars", label: "n 文字目より n 文字を削除" },
      { kind: "delete_before", label: "n 文字目より前を削除" },
      { kind: "delete_pattern", label: "決まったパターンを削除" },
      { kind: "make_83", label: "8.3 形式に丸める" },
    ],
  },
  {
    title: "文字種の変換",
    items: [
      { kind: "case_convert", label: "英字の大文字小文字を変換" },
      { kind: "kana_convert", label: "仮名を変換（ひら⇔カタ・全⇔半）" },
      { kind: "width_convert", label: "全角⇔半角（英数記号）" },
      { kind: "clear_diacritics", label: "ダイアクリティカル除去" },
      { kind: "remove_voiced_mark", label: "濁音・半濁音を除去" },
    ],
  },
  {
    title: "文字列の置換",
    items: [{ kind: "string_replace", label: "文字列置換" }],
  },
  {
    title: "数値の整理",
    items: [
      { kind: "number_pad", label: "n 番目の数値の桁合わせ" },
      { kind: "number_adjust", label: "n 番目の数値を増減" },
    ],
  },
  {
    title: "拡張子の変換",
    items: [
      { kind: "ext_convert", label: "拡張子の大文字小文字を変換" },
      { kind: "ext_delete", label: "拡張子を削除" },
      { kind: "ext_add", label: "拡張子を追加" },
      { kind: "ext_replace", label: "拡張子を置換" },
    ],
  },
];

/// 各 kind の初期パラメータ。BuiltinMenu でユーザが選択した瞬間に
/// この値が初期 op として App.tsx に反映される。
export function initialOp(kind: BuiltinKind): BuiltinOp {
  switch (kind) {
    case "add_seq_str":
      return { type: "add_seq_str", prefix: "", suffix: "", digits: 3, start: 0, step: 1 };
    case "add_datetime":
      return { type: "add_datetime", format: "\\Y\\m\\d", position: "prefix" };
    case "add_folder_name":
      return { type: "add_folder_name", position: "prefix" };
    case "add_folder_seq":
      return { type: "add_folder_seq", digits: 3, start: 0, step: 1 };
    case "truncate_from_start":
      return { type: "truncate_from_start", n: 1 };
    case "truncate_from_end":
      return { type: "truncate_from_end", n: 1 };
    case "delete_chars":
      return { type: "delete_chars", from: "start", offset: 0, count: 1 };
    case "delete_before":
      return { type: "delete_before", from: "start", n: 1 };
    case "delete_pattern":
      return { type: "delete_pattern", pattern: "copy_num" };
    case "make_83":
      return { type: "make_83" };
    case "case_convert":
      return { type: "case_convert", conversion: "upper", skip_ext: true };
    case "kana_convert":
      return { type: "kana_convert", conversion: "hira_to_kata", skip_ext: true };
    case "width_convert":
      return { type: "width_convert", conversion: "to_hankaku", skip_ext: true };
    case "clear_diacritics":
      return { type: "clear_diacritics", skip_ext: true };
    case "remove_voiced_mark":
      return { type: "remove_voiced_mark", skip_ext: true };
    case "string_replace":
      return { type: "string_replace", search: "", replace: "" };
    case "number_pad":
      return { type: "number_pad", from: "start", n: 1, digits: 3 };
    case "number_adjust":
      return { type: "number_adjust", from: "start", n: 1, delta: 1 };
    case "ext_convert":
      return { type: "ext_convert", conversion: "lower" };
    case "ext_delete":
      return { type: "ext_delete" };
    case "ext_add":
      return { type: "ext_add", ext: "" };
    case "ext_replace":
      return { type: "ext_replace", ext: "" };
  }
}
