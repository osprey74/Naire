use std::path::Path;
use std::sync::OnceLock;

use chrono::format::Locale;
use chrono::{DateTime, Datelike, Local, Timelike};

use crate::rename::sequence::{format_seq, Numbering};

/// 1 ファイルに対するリネームで参照されるコンテキスト。
///
/// **重要**: `original_full` は「マクロ開始時点」または「ステップ非進行時の元名」を
/// 保持する。非マクロ単一ステップでは `current_name == original_full` となるが、
/// マクロのステップ処理では `current` 引数が各ステップで変化する一方、
/// `original_full` は変わらない（`\orig` 変数で参照される）。
///
/// `\0 \t \e` は **`current` 引数**から導出し、`\orig` のみ `original_full` を参照する。
pub struct FileContext {
    /// マクロ開始時の元ファイル名（拡張子含む）。`\orig` で参照。
    pub original_full: String,
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

/// OS のロケール（BCP 47, 例: `ja-JP`、`en_US.UTF-8`）を chrono の `Locale` に正規化する。
/// 不一致なら POSIX にフォールバック。初回呼び出しで取得した値を OnceLock でキャッシュ。
fn current_locale() -> Locale {
    static CACHED: OnceLock<Locale> = OnceLock::new();
    *CACHED.get_or_init(|| {
        let raw = sys_locale::get_locale().unwrap_or_else(|| "en_US".to_string());
        let head = raw.split(['.', '@']).next().unwrap_or("en_US");
        let normalized = head.replace('-', "_");
        Locale::try_from(normalized.as_str()).unwrap_or(Locale::POSIX)
    })
}

/// `\u` `\U` `\l` `\L` `\E` の状態を保持する case 修飾モード。
#[derive(Clone, Copy, PartialEq, Eq)]
enum CaseMode {
    None,
    /// `\u`: 直後 1 文字を upper にしたら自動的に None に戻る
    NextUpper,
    /// `\l`: 直後 1 文字を lower にしたら自動的に None に戻る
    NextLower,
    /// `\U` ... `\E`
    AllUpper,
    /// `\L` ... `\E`
    AllLower,
}

/// 出力アキュムレータ。`push_*` を経由することで case 修飾子を全文字に対して適用する。
/// 文字単位で適用するため、変数展開や regex キャプチャに対しても自然に効く
/// （Perl / Boost.Regex の `\u\1` と同様の振る舞い）。
struct CaseAcc {
    out: String,
    mode: CaseMode,
}

impl CaseAcc {
    fn with_capacity(cap: usize) -> Self {
        Self {
            out: String::with_capacity(cap),
            mode: CaseMode::None,
        }
    }

    fn push_char(&mut self, c: char) {
        match self.mode {
            CaseMode::None => self.out.push(c),
            CaseMode::NextUpper => {
                for u in c.to_uppercase() {
                    self.out.push(u);
                }
                self.mode = CaseMode::None;
            }
            CaseMode::NextLower => {
                for u in c.to_lowercase() {
                    self.out.push(u);
                }
                self.mode = CaseMode::None;
            }
            CaseMode::AllUpper => {
                for u in c.to_uppercase() {
                    self.out.push(u);
                }
            }
            CaseMode::AllLower => {
                for u in c.to_lowercase() {
                    self.out.push(u);
                }
            }
        }
    }

    fn push_str(&mut self, s: &str) {
        for c in s.chars() {
            self.push_char(c);
        }
    }

    fn set_mode(&mut self, m: CaseMode) {
        self.mode = m;
    }

    fn finish(self) -> String {
        self.out
    }
}

/// 現在の名前を stem / extension に分解する（最後の `.` で分割）。
fn split_stem_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) => (&name[..i], &name[i..]),
        None => (name, ""),
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
/// `strip_zero=true` で `\#X` 修飾子相当の振る舞いになる。ロケール依存変数
/// (`\a \A \b \B \p`) は `chrono::format_localized` 経由で OS ロケールに従う。
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
        'a' => ctx.mtime.format_localized("%a", current_locale()).to_string(),
        'A' => ctx.mtime.format_localized("%A", current_locale()).to_string(),
        'b' => ctx.mtime.format_localized("%b", current_locale()).to_string(),
        'B' => ctx.mtime.format_localized("%B", current_locale()).to_string(),
        'p' => ctx.mtime.format_localized("%p", current_locale()).to_string(),
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
/// - `\a` 曜日省略名 / `\A` 曜日完全名（OS ロケール依存）
/// - `\b` 月省略名 / `\B` 月完全名（OS ロケール依存）
/// - `\p` AM/PM 表記（OS ロケール依存）
/// - `\#X` 次の日時変数の先行ゼロを除去
/// - `\u` 次の 1 文字を大文字化 / `\l` 次の 1 文字を小文字化
/// - `\U` ... `\E` で囲まれた範囲を大文字化 / `\L` ... `\E` で小文字化
/// - `\E` `\U` / `\L` の効果を終了
/// - `\1`〜`\9` キャプチャグループ（`caps` 指定時のみ）
/// - `\orig` マクロ開始時点の元ファイル名（マクロ専用）
/// - `?` `??` `???` `????` 連番（`ctx.seq_value` を `ctx.seq_numbering` で書式化）
pub fn expand_template(
    template: &str,
    caps: Option<&regex::Captures>,
    ctx: &FileContext,
    current: &str,
) -> String {
    let chars: Vec<char> = template.chars().collect();
    let mut acc = CaseAcc::with_capacity(template.len());
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            let nxt = chars[i + 1];

            // `\orig` (5 文字消費): マクロ開始時点の元ファイル名。
            // 非マクロ単一ステップでは `current == ctx.original_full` のため
            // `\0` と同じ結果になる。
            if nxt == 'o' && chars.get(i + 2..i + 5) == Some(&['r', 'i', 'g']) {
                acc.push_str(&ctx.original_full);
                i += 5;
                continue;
            }

            // `\#X` (3 文字消費): 次の日時変数の先行ゼロを除去
            if nxt == '#' && i + 2 < chars.len() {
                if let Some(v) = format_date_var(chars[i + 2], ctx, true) {
                    acc.push_str(&v);
                    i += 3;
                    continue;
                }
                acc.push_char('\\');
                acc.push_char('#');
                i += 2;
                continue;
            }
            match nxt {
                '\\' => acc.push_char('\\'),
                '?' => acc.push_char('?'),
                '0' => acc.push_str(current),
                't' => {
                    let (stem, _) = split_stem_ext(current);
                    acc.push_str(stem);
                }
                'e' => {
                    let (_, ext) = split_stem_ext(current);
                    acc.push_str(ext);
                }
                'f' => acc.push_str(&ctx.folder_name),
                'F' => acc.push_str(&ctx.parent_folder_name),
                ';' => acc.push_str(&format_size_with_sep(ctx.size)),
                ':' => acc.push_str(&ctx.size.to_string()),
                'Y' | 'y' | 'm' | 'd' | 'H' | 'I' | 'M' | 'S' | 'a' | 'A' | 'b' | 'B'
                | 'p' => {
                    if let Some(v) = format_date_var(nxt, ctx, false) {
                        acc.push_str(&v);
                    }
                }
                'u' => acc.set_mode(CaseMode::NextUpper),
                'l' => acc.set_mode(CaseMode::NextLower),
                'U' => acc.set_mode(CaseMode::AllUpper),
                'L' => acc.set_mode(CaseMode::AllLower),
                'E' => acc.set_mode(CaseMode::None),
                d @ '1'..='9' => {
                    let idx = d.to_digit(10).unwrap() as usize;
                    if let Some(caps) = caps {
                        if let Some(m) = caps.get(idx) {
                            acc.push_str(m.as_str());
                        }
                    }
                }
                _ => {
                    acc.push_char('\\');
                    acc.push_char(nxt);
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
            acc.push_str(&format_seq(ctx.seq_value, digits, ctx.seq_numbering));
            i = j;
        } else {
            acc.push_char(c);
            i += 1;
        }
    }
    acc.finish()
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
        assert_eq!(c.folder_name, "folder");
        assert_eq!(c.parent_folder_name, "parent");
    }

    #[test]
    fn template_capture_groups() {
        let re = regex::Regex::new(r"^(\w+)_(\d+)$").unwrap();
        let caps = dummy_caps(&re, "abc_123");
        let c = ctx("/x/folder/file.txt");
        assert_eq!(
            expand_template(r"\2-\1", Some(&caps), &c, "file.txt"),
            "123-abc"
        );
    }

    #[test]
    fn template_file_vars() {
        let re = regex::Regex::new(r".*").unwrap();
        let caps = dummy_caps(&re, "file.txt");
        let c = ctx("/x/folder/file.txt");
        assert_eq!(
            expand_template(r"\f_\t\e", Some(&caps), &c, "file.txt"),
            "folder_file.txt"
        );
    }

    #[test]
    fn template_no_captures() {
        let c = ctx("/x/folder/file.txt");
        assert_eq!(
            expand_template(r"\f_\t\e", None, &c, "file.txt"),
            "folder_file.txt"
        );
        assert_eq!(expand_template(r"\1", None, &c, "file.txt"), "");
    }

    #[test]
    fn template_size_vars() {
        let mut c = ctx("/x/folder/file.txt");
        c.size = 1_234_567;
        assert_eq!(expand_template(r"\;", None, &c, "file.txt"), "1,234,567");
        assert_eq!(expand_template(r"\:", None, &c, "file.txt"), "1234567");
    }

    #[test]
    fn template_date_vars() {
        let dt = Local.with_ymd_and_hms(2026, 5, 12, 9, 8, 7).unwrap();
        let c = ctx_with_dt("/x/folder/file.txt", dt);
        assert_eq!(
            expand_template(r"\Y\m\d_\H\M\S", None, &c, "file.txt"),
            "20260512_090807"
        );
        assert_eq!(expand_template(r"\y", None, &c, "file.txt"), "26");
        assert_eq!(expand_template(r"\I", None, &c, "file.txt"), "09");
    }

    #[test]
    fn template_strip_zero_modifier() {
        let dt = Local.with_ymd_and_hms(2026, 5, 7, 0, 5, 0).unwrap();
        let c = ctx_with_dt("/x/file.txt", dt);
        assert_eq!(expand_template(r"\#m", None, &c, "file.txt"), "5");
        assert_eq!(expand_template(r"\#d", None, &c, "file.txt"), "7");
        assert_eq!(expand_template(r"\#H", None, &c, "file.txt"), "0");
        assert_eq!(expand_template(r"\#m\d", None, &c, "file.txt"), "507");
    }

    #[test]
    fn template_backslash_escape() {
        let re = regex::Regex::new(r".*").unwrap();
        let caps = dummy_caps(&re, "file.txt");
        let c = ctx("/x/folder/file.txt");
        assert_eq!(
            expand_template(r"\\foo", Some(&caps), &c, "file.txt"),
            r"\foo"
        );
    }

    #[test]
    fn unknown_escape_preserved() {
        let c = ctx("/x/folder/file.txt");
        // `\p` (ロケール) と `\u` (case 修飾) は実装済みなので `\q` で検証
        assert_eq!(
            expand_template(r"\q_\t", None, &c, "file.txt"),
            r"\q_file"
        );
    }

    #[test]
    fn template_sequence_decimal() {
        let mut c = ctx("/x/folder/file.txt");
        c.seq_value = 7;
        c.seq_numbering = Numbering::Decimal;
        assert_eq!(
            expand_template("img_???", None, &c, "file.txt"),
            "img_007"
        );
        assert_eq!(expand_template("?", None, &c, "file.txt"), "7");
        assert_eq!(expand_template("????", None, &c, "file.txt"), "0007");
    }

    #[test]
    fn template_sequence_hex() {
        let mut c = ctx("/x/folder/file.txt");
        c.seq_value = 255;
        c.seq_numbering = Numbering::Hex;
        assert_eq!(expand_template("??", None, &c, "file.txt"), "FF");
    }

    #[test]
    fn template_sequence_alpha() {
        // alpha は bijective: value=26 → "AA"（digits 指定は無視）
        let mut c = ctx("/x/folder/file.txt");
        c.seq_value = 26;
        c.seq_numbering = Numbering::Alpha;
        assert_eq!(expand_template("??", None, &c, "file.txt"), "AA");
        c.seq_value = 0;
        assert_eq!(expand_template("?", None, &c, "file.txt"), "A");
    }

    #[test]
    fn template_literal_question_mark() {
        let c = ctx("/x/folder/file.txt");
        assert_eq!(
            expand_template(r"foo\?bar", None, &c, "file.txt"),
            "foo?bar"
        );
    }

    #[test]
    fn template_consecutive_questions_capped_at_4() {
        let mut c = ctx("/x/folder/file.txt");
        c.seq_value = 5;
        assert_eq!(expand_template("?????", None, &c, "file.txt"), "00055");
    }

    #[test]
    fn template_orig_macro_variable() {
        // マクロ context: original_full は step 0 名、current は現在ステップ入力
        let mut c = ctx("/x/folder/original.txt");
        c.original_full = "original.txt".into();
        // 現在ステップでの \0 と \t \e は current から
        assert_eq!(
            expand_template(r"\orig", None, &c, "current_v3.md"),
            "original.txt"
        );
        assert_eq!(expand_template(r"\0", None, &c, "current_v3.md"), "current_v3.md");
        assert_eq!(expand_template(r"\t", None, &c, "current_v3.md"), "current_v3");
        assert_eq!(expand_template(r"\e", None, &c, "current_v3.md"), ".md");
        // \orig + \1 \orig 連結ユースケース
        assert_eq!(
            expand_template(r"prefix \orig suffix", None, &c, "x"),
            "prefix original.txt suffix"
        );
    }

    // ── case 修飾子 ───────────────────────────────────────────
    #[test]
    fn template_case_next_upper() {
        let c = ctx("/x/file.txt");
        assert_eq!(
            expand_template(r"\u\t", None, &c, "abc.md"),
            "Abc"
        );
        // \u は直後 1 文字のみに作用
        assert_eq!(
            expand_template(r"\uabc", None, &c, "x.md"),
            "Abc"
        );
    }

    #[test]
    fn template_case_next_lower() {
        let c = ctx("/x/file.txt");
        assert_eq!(
            expand_template(r"\l\t", None, &c, "ABC.MD"),
            "aBC"
        );
    }

    #[test]
    fn template_case_all_upper_until_E() {
        let c = ctx("/x/file.txt");
        assert_eq!(
            expand_template(r"\Uabc\E def", None, &c, "x.md"),
            "ABC def"
        );
    }

    #[test]
    fn template_case_all_lower_until_E() {
        let c = ctx("/x/file.txt");
        assert_eq!(
            expand_template(r"\LFOO\EBAR", None, &c, "x.md"),
            "fooBAR"
        );
    }

    #[test]
    fn template_case_composes_with_capture() {
        let re = regex::Regex::new(r"^(\w+)$").unwrap();
        let caps = dummy_caps(&re, "abc");
        let c = ctx("/x/file.txt");
        assert_eq!(
            expand_template(r"\u\1", Some(&caps), &c, "file.txt"),
            "Abc"
        );
    }

    #[test]
    fn template_case_composes_with_var() {
        let c = ctx("/x/folder/file.txt");
        // \U\f\E でフォルダ名を全大文字化
        assert_eq!(
            expand_template(r"\U\f\E.dat", None, &c, "file.txt"),
            "FOLDER.dat"
        );
    }

    #[test]
    fn template_case_unicode_japanese_passthrough() {
        // 日本語文字は upper/lower で変化しないことを確認（崩れない）
        let c = ctx("/x/file.txt");
        assert_eq!(
            expand_template(r"\Uあいう\E", None, &c, "x.md"),
            "あいう"
        );
    }

    // ── ロケール変数 ──────────────────────────────────────────
    // 文字列内容はロケール依存のため、`\a` 等が「リテラルでないこと」と
    // 「日時情報を含む空でない文字列を返すこと」のみ検証する。
    #[test]
    fn template_locale_vars_recognized() {
        let dt = Local.with_ymd_and_hms(2026, 5, 12, 14, 0, 0).unwrap();
        let c = ctx_with_dt("/x/file.txt", dt);
        let a = expand_template(r"\a", None, &c, "file.txt");
        let aa = expand_template(r"\A", None, &c, "file.txt");
        let b = expand_template(r"\b", None, &c, "file.txt");
        let bb = expand_template(r"\B", None, &c, "file.txt");
        let p = expand_template(r"\p", None, &c, "file.txt");
        // リテラル `\a` `\A` `\b` `\B` `\p` が出てこないこと
        assert!(!a.starts_with('\\'));
        assert!(!aa.starts_with('\\'));
        assert!(!b.starts_with('\\'));
        assert!(!bb.starts_with('\\'));
        assert!(!p.starts_with('\\'));
        assert!(!a.is_empty());
        assert!(!aa.is_empty());
        assert!(!b.is_empty());
        assert!(!bb.is_empty());
        assert!(!p.is_empty());
    }
}
