export type SupportContext =
  | "wildcard.search"
  | "wildcard.replace"
  | "regex.search"
  | "regex.replace"
  | "char.search"
  | "char.replace";

export interface SupportItem {
  label: string;
  insert: string;
}

export interface SupportGroup {
  title: string;
  items: SupportItem[];
}

// HANDOFF の「置換変数仕様」「サポートボタン仕様」に従って定義。
// Phase 5 では regex.* のみ実装。他コンテキストは対応 Phase で追加。
const REGEX_SEARCH: SupportGroup[] = [
  {
    title: "標準",
    items: [
      { label: "任意の1文字", insert: "." },
      { label: "任意の文字列", insert: ".*" },
    ],
  },
  {
    title: "文字クラス",
    items: [
      { label: "A から Z の範囲内", insert: "[A-Z]" },
      { label: "A から Z の範囲外", insert: "[^A-Z]" },
      { label: "数字", insert: "\\d" },
      { label: "数字以外", insert: "\\D" },
      { label: "単語", insert: "\\w" },
      { label: "単語以外", insert: "\\W" },
      { label: "空白", insert: "\\s" },
      { label: "空白以外", insert: "\\S" },
    ],
  },
  {
    title: "メタ文字",
    items: [
      { label: "ピリオド", insert: "\\." },
      { label: "先頭", insert: "^" },
      { label: "末尾", insert: "$" },
      { label: "または", insert: "|" },
      { label: "直前の1文字、または空文字", insert: "?" },
      { label: "0回以上の繰り返し", insert: "*" },
      { label: "1回以上の繰り返し", insert: "+" },
      { label: "m回の繰り返し", insert: "{m}" },
      { label: "m回以上の繰り返し", insert: "{m,}" },
      { label: "m回以上n回以内の繰り返し", insert: "{m,n}" },
    ],
  },
  {
    title: "グループ",
    items: [{ label: "グループ化・キャプチャ", insert: "()" }],
  },
];

// regex.replace と wildcard.replace は共通仕様（HANDOFF）。
// マクロ専用 \\orig は Phase 11 で別 menu として追加する。
const REPLACE_COMMON: SupportGroup[] = [
  {
    title: "ファイル",
    items: [
      { label: "現在の名前", insert: "\\0" },
      { label: "現在のファイルタイトル", insert: "\\t" },
      { label: "現在の拡張子", insert: "\\e" },
      { label: "フォルダ名", insert: "\\f" },
      { label: "親フォルダ名", insert: "\\F" },
      { label: "ファイルサイズ（桁区切りあり）", insert: "\\;" },
      { label: "ファイルサイズ（桁区切りなし）", insert: "\\:" },
    ],
  },
  {
    title: "連番",
    items: [
      { label: "1桁連番", insert: "?" },
      { label: "2桁連番", insert: "??" },
      { label: "3桁連番", insert: "???" },
      { label: "4桁連番", insert: "????" },
    ],
  },
  {
    title: "日時（ファイル mtime）",
    items: [
      { label: "ファイル日付（4桁西暦+月+日）", insert: "\\Y\\m\\d" },
      { label: "ファイル日付（2桁西暦+月+日）", insert: "\\y\\m\\d" },
      { label: "ファイル時刻（24時+分+秒）", insert: "\\H\\M\\S" },
      { label: "ファイル時刻（12時+分+秒）", insert: "\\I\\M\\S" },
      { label: "ファイル時刻(午前午後+時+分+秒)", insert: "\\p\\H\\M\\S" },
      { label: "曜日（省略名）", insert: "\\a" },
      { label: "曜日（正式名）", insert: "\\A" },
      { label: "月名（省略名）", insert: "\\b" },
      { label: "月名（正式名）", insert: "\\B" },
      { label: "先行ゼロ削除（次の日時変数に適用）", insert: "\\#" },
    ],
  },
  {
    title: "キャプチャ",
    items: [
      { label: "グループ1にマッチ", insert: "\\1" },
      { label: "グループ2にマッチ", insert: "\\2" },
      { label: "グループ3にマッチ", insert: "\\3" },
      { label: "グループ4にマッチ", insert: "\\4" },
      { label: "グループ5にマッチ", insert: "\\5" },
      { label: "グループ6にマッチ", insert: "\\6" },
      { label: "グループ7にマッチ", insert: "\\7" },
      { label: "グループ8にマッチ", insert: "\\8" },
      { label: "グループ9にマッチ", insert: "\\9" },
    ],
  },
  {
    title: "大文字小文字制御",
    items: [
      { label: "次にくる1文字を大文字", insert: "\\u" },
      { label: "次にくる文字から \\E までを大文字", insert: "\\U" },
      { label: "次にくる1文字を小文字", insert: "\\l" },
      { label: "次にくる文字から \\E までを小文字", insert: "\\L" },
      { label: "\\U \\L の効果終了", insert: "\\E" },
    ],
  },
];

const WILDCARD_SEARCH: SupportGroup[] = [
  {
    title: "ワイルドカード",
    items: [
      { label: "任意の1文字", insert: "?" },
      { label: "任意の文字列", insert: "*" },
    ],
  },
];

// char.search / char.replace 共通。範囲記法（`a-z`）と列挙（カンマ区切り）の
// プリセット集合。挿入後、ユーザがそのまま使うか手動で組み合わせる想定。
const CHAR_PRESETS: SupportGroup[] = [
  {
    title: "英字",
    items: [
      { label: "小文字", insert: "a-z" },
      { label: "大文字", insert: "A-Z" },
      { label: "全角小文字", insert: "ａ-ｚ" },
      { label: "全角大文字", insert: "Ａ-Ｚ" },
    ],
  },
  {
    title: "仮名",
    items: [
      { label: "ひらがな", insert: "あ-ん" },
      { label: "カタカナ", insert: "ア-ン" },
    ],
  },
  {
    title: "数字",
    items: [
      { label: "数字", insert: "0-9" },
      { label: "全角数字", insert: "０-９" },
      { label: "漢数字", insert: "〇一二三四五六七八九十" },
      { label: "大字表記", insert: "〇壱弐参四五六七八九拾" },
    ],
  },
  {
    title: "ローマ数字",
    items: [
      {
        label: "ローマ数字[小]",
        insert: "0,i,ii,iii,iv,v,vi,vii,viii,ix,x",
      },
      {
        label: "ローマ数字[大]",
        insert: "0,I,II,III,IV,V,VI,VII,VIII,IX,X",
      },
    ],
  },
];

export const SUPPORT_MENUS: Record<SupportContext, SupportGroup[]> = {
  "regex.search": REGEX_SEARCH,
  "regex.replace": REPLACE_COMMON,
  "wildcard.search": WILDCARD_SEARCH,
  "wildcard.replace": REPLACE_COMMON,
  "char.search": CHAR_PRESETS,
  "char.replace": CHAR_PRESETS,
};
