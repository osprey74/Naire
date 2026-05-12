use std::path::Path;

use chrono::{DateTime, Datelike, Local, Timelike};

use crate::rename::sequence::{format_seq, Numbering};

/// 1 ファイルに対するリネームで参照される静的コンテキスト。
/// 置換テンプレート内の `\0 \t \e \f \F \Y \m \d ?` などを解決するためにステップ前に構築する。
pub struct FileContext {
    pub original_full: String,
    pub stem: String,
    pub extension: String,
    pub folder_name: String,
    pub parent_folder_name: String,
    pub size: u64,
    pub mtime: DateTime<Local>,
    /// 連番 `?` `??` `???` `????` で参照される値（advanced の global counter）。
    pub seq_value: u64,
    pub seq_numbering: Numbering,
    /// 0 始まり: build_preview がステップ対象行を昇順走査する際の通し番号。
    /// 定型の `add_seq_str` / `add_folder_seq` が op 個別の `start + step * applied_index`
    /// を計算するために使う。
    pub applied_index: u64,
}

impl FileContext {
    pub fn from_path(path: &Path) -> Self {
        let original_full = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let extension = path
            .extension()
            .map(|s| format!(".{}", s.to_string_lossy()))
            .unwrap_or_default();
        let folder_name = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let parent_folder_name = path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self {
            original_full,
            stem,
            extension,
            folder_name,
            parent_folder_name,
            size: 0,
            mtime: Local::now(),
            seq_value: 0,
            seq_numbering: Numbering::Decimal,
            applied_index: 0,
        }
    }
}

/// ファイルサイズを 3 桁区切りでフォーマット（`1,234,567`）。
fn format_size_with_sep(size: u64) -> String {
    let s = size.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        let from_end = bytes.len() - i;
        if i > 0 && from_end % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

/// 日時系の `\#X` 修飾子適用後の値を文字列化する。
/// `strip_zero=true` のときは先頭ゼロを取り除く（`05` → `5`、`00` → `0`）。
fn format_dt_value(s: String, strip_zero: bool) -> String {
    if !strip_zero {
        return s;
    }
    let trimmed = s.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".into()
    } else {
        trimmed.into()
    }
}

/// 日時変数 1 文字を `ctx.mtime` から書式化する。未知の letter なら `None`。
/// `strip_zero=true` で `\#X` 修飾子相当の振る舞いになる。
fn format_date_var(letter: char, ctx: &FileContext, strip_zero: bool) -> Option<String> {
    let raw = match letter {
        'Y' => format!("{:04}", ctx.mtime.year()),
        'y' => format!("{:02}", ctx.mtime.year() % 100),
        'm' => format!("{:02}", ctx.mtime.month()),
        'd' => format!("{:02}", ctx.mtime.day()),
        'H' => format!("{:02}", ctx.mtime.hour()),
        'I' => format!("{:02}", ctx.mtime.hour12().1),
        'M' => format!("{:02}", ctx.mtime.minute()),
        'S' => format!("{:02}", ctx.mtime.second()),
        _ => return None,
    };
    Some(format_dt_value(raw, strip_zero))
}

/// 置換テンプレートを 1 文字ずつ走査して、定義済みのバックスラッシュ変数、
/// キャプチャグループ `\1`〜`\9`、連番 `?` `??` `???` `????`、日時系を展開する。
///
/// 対応する変数:
///
/// - `\\` リテラルバックスラッシュ
/// - `\?` リテラル `?`
/// - `\0` 現在のファイル名（拡張子含む）
/// - `\t` ファイルタイトル（拡張子除く）
/// - `\e` 拡張子（ドット含む）
/// - `\f` 所属フォルダ名
/// - `\F` 親フォルダ名
/// - `\;` ファイルサイズ（3 桁区切り）
/// - `\:` ファイルサイズ（区切りなし）
/// - `\Y` 4 桁西暦 / `\y` 2 桁西暦
/// - `\m` 月（01-12）/ `\d` 日（01-31）
/// - `\H` 時（24h, 00-23）/ `\I` 時（12h, 01-12）
/// - `\M` 分（00-59）/ `\S` 秒（00-59）
/// - `\#X` 次の日時変数の先行ゼロを除去
/// - `\1`〜`\9` キャプチャグループ（`caps` 指定時のみ）
/// - `?` `??` `???` `????` 連番（`ctx.seq_value` を `ctx.seq_numbering` で書式化）
///
/// ロケール依存変数（`\a \A \b \B \p`）と大文字小文字制御（`\u \U \l \L \E`）は未実装で
/// リテラル出力する。マクロ専用 `\orig` も Phase 11 で実装。
pub fn expand_template(
    template: &str,
    caps: Option<&regex::Captures>,
    ctx: &FileContext,
) -> String {
    let chars: Vec<char> = template.chars().collect();
    let mut out = String::with_capacity(template.len());
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            let nxt = chars[i + 1];
            // `\#X` (3 文字消費): 次の日時変数の先行ゼロを除去
            if nxt == '#' && i + 2 < chars.len() {
                if let Some(v) = format_date_var(chars[i + 2], ctx, true) {
                    out.push_str(&v);
                    i += 3;
                    continue;
                }
                // 日時変数でなければ `\#` をリテラル出力
                out.push('\\');
                out.push('#');
                i += 2;
                continue;
            }
            match nxt {
                '\\' => out.push('\\'),
                '?' => out.push('?'),
                '0' => out.push_str(&ctx.original_full),
                't' => out.push_str(&ctx.stem),
                'e' => out.push_str(&ctx.extension),
                'f' => out.push_str(&ctx.folder_name),
                'F' => out.push_str(&ctx.parent_folder_name),
                ';' => out.push_str(&format_size_with_sep(ctx.size)),
                ':' => out.push_str(&ctx.size.to_string()),
                'Y' | 'y' | 'm' | 'd' | 'H' | 'I' | 'M' | 'S' => {
                    if let Some(v) = format_date_var(nxt, ctx, false) {
                        out.push_str(&v);
                    }
                }
                d @ '1'..='9' => {
                    let idx = d.to_digit(10).unwrap() as usize;
                    if let Some(caps) = caps {
                        if let Some(m) = caps.get(idx) {
                            out.push_str(m.as_str());
                        }
                    }
                }
                _ => {
                    out.push('\\');
                    out.push(nxt);
                }
            }
            i += 2;
        } else if c == '?' {
            let mut digits = 1;
            let mut j = i + 1;
            while j < chars.len() && chars[j] == '?' && digits < 4 {
                digits += 1;
                j += 1;
            }
            out.push_str(&format_seq(ctx.seq_value, digits, ctx.seq_numbering));
            i = j;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::path::PathBuf;

    fn ctx(path: &str) -> FileContext {
        FileContext::from_path(&PathBuf::from(path))
    }

    fn ctx_with_dt(path: &str, dt: DateTime<Local>) -> FileContext {
        let mut c = ctx(path);
        c.mtime = dt;
        c
    }

    fn dummy_caps<'a>(re: &'a regex::Regex, text: &'a str) -> regex::Captures<'a> {
        re.captures(text).unwrap()
    }

    #[test]
    fn file_context_basic() {
        let c = ctx("/root/parent/folder/file.txt");
        assert_eq!(c.original_full, "file.txt");
        assert_eq!(c.stem, "file");
        assert_eq!(c.extension, ".txt");
        assert_eq!(c.folder_name, "folder");
        assert_eq!(c.parent_folder_name, "parent");
    }

    #[test]
    fn template_capture_groups() {
        let re = regex::Regex::new(r"^(\w+)_(\d+)$").unwrap();
        let caps = dummy_caps(&re, "abc_123");
        let c = ctx("/x/folder/file.txt");
        assert_eq!(expand_template(r"\2-\1", Some(&caps), &c), "123-abc");
    }

    #[test]
    fn template_file_vars() {
        let re = regex::Regex::new(r".*").unwrap();
        let caps = dummy_caps(&re, "file.txt");
        let c = ctx("/x/folder/file.txt");
        assert_eq!(
            expand_template(r"\f_\t\e", Some(&caps), &c),
            "folder_file.txt"
        );
    }

    #[test]
    fn template_no_captures() {
        let c = ctx("/x/folder/file.txt");
        // caps=None でも file 変数は展開可能
        assert_eq!(expand_template(r"\f_\t\e", None, &c), "folder_file.txt");
        // \1 は何も出ない
        assert_eq!(expand_template(r"\1", None, &c), "");
    }

    #[test]
    fn template_size_vars() {
        let mut c = ctx("/x/folder/file.txt");
        c.size = 1_234_567;
        assert_eq!(expand_template(r"\;", None, &c), "1,234,567");
        assert_eq!(expand_template(r"\:", None, &c), "1234567");
    }

    #[test]
    fn template_date_vars() {
        let dt = Local.with_ymd_and_hms(2026, 5, 12, 9, 8, 7).unwrap();
        let c = ctx_with_dt("/x/folder/file.txt", dt);
        assert_eq!(
            expand_template(r"\Y\m\d_\H\M\S", None, &c),
            "20260512_090807"
        );
        assert_eq!(expand_template(r"\y", None, &c), "26");
        assert_eq!(expand_template(r"\I", None, &c), "09");
    }

    #[test]
    fn template_strip_zero_modifier() {
        let dt = Local.with_ymd_and_hms(2026, 5, 7, 0, 5, 0).unwrap();
        let c = ctx_with_dt("/x/file.txt", dt);
        assert_eq!(expand_template(r"\#m", None, &c), "5");
        assert_eq!(expand_template(r"\#d", None, &c), "7");
        assert_eq!(expand_template(r"\#H", None, &c), "0");
        // 修飾子は次の 1 個だけに作用（後続の \d はゼロ埋めのまま）
        assert_eq!(expand_template(r"\#m\d", None, &c), "507");
    }

    #[test]
    fn template_backslash_escape() {
        let re = regex::Regex::new(r".*").unwrap();
        let caps = dummy_caps(&re, "file.txt");
        let c = ctx("/x/folder/file.txt");
        assert_eq!(expand_template(r"\\foo", Some(&caps), &c), r"\foo");
    }

    #[test]
    fn unknown_escape_preserved() {
        let c = ctx("/x/folder/file.txt");
        // \p \a \A \b \B \u \U \l \L \E は Phase 8 では未対応
        assert_eq!(expand_template(r"\p_\t", None, &c), r"\p_file");
    }

    #[test]
    fn template_sequence_decimal() {
        let mut c = ctx("/x/folder/file.txt");
        c.seq_value = 7;
        c.seq_numbering = Numbering::Decimal;
        assert_eq!(expand_template("img_???", None, &c), "img_007");
        assert_eq!(expand_template("?", None, &c), "7");
        assert_eq!(expand_template("????", None, &c), "0007");
    }

    #[test]
    fn template_sequence_hex() {
        let mut c = ctx("/x/folder/file.txt");
        c.seq_value = 255;
        c.seq_numbering = Numbering::Hex;
        assert_eq!(expand_template("??", None, &c), "FF");
    }

    #[test]
    fn template_sequence_alpha() {
        let mut c = ctx("/x/folder/file.txt");
        c.seq_value = 26;
        c.seq_numbering = Numbering::Alpha;
        assert_eq!(expand_template("??", None, &c), "BA");
    }

    #[test]
    fn template_literal_question_mark() {
        let c = ctx("/x/folder/file.txt");
        assert_eq!(expand_template(r"foo\?bar", None, &c), "foo?bar");
    }

    #[test]
    fn template_consecutive_questions_capped_at_4() {
        let mut c = ctx("/x/folder/file.txt");
        c.seq_value = 5;
        assert_eq!(expand_template("?????", None, &c), "00055");
    }
}
