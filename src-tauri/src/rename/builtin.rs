use serde::{Deserialize, Serialize};

use crate::rename::convert;
use crate::rename::sequence::format_seq;
use crate::rename::variables::{expand_template, FileContext};

// ── 補助 enum ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    Prefix,
    Suffix,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Start,
    End,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeletePattern {
    CopyNum,
    CopyNumVista,
    Shortcut,
    ShortcutVista,
    BracketContent,
    NumKagiOrRound,
    NumKagi,
    NumRound,
    NumLenticular,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CaseConversion {
    Capitalize,
    Upper,
    Lower,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KanaConversion {
    HiraToKata,
    KataToHira,
    HankakuKataToZenkaku,
    ZenkakuKataToHankaku,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WidthConversion {
    ToZenkaku,
    ToHankaku,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExtConversion {
    Upper,
    Lower,
}

// ── 定型操作 ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BuiltinOp {
    // 連番・文字列の追加
    AddSeqStr {
        prefix: String,
        suffix: String,
        digits: usize,
        start: u64,
        step: u64,
    },
    AddDatetime {
        format: String,
        position: Position,
    },
    AddFolderName {
        position: Position,
    },
    AddFolderSeq {
        digits: usize,
        start: u64,
        step: u64,
    },
    TruncateFromStart {
        n: usize,
    },
    TruncateFromEnd {
        n: usize,
    },
    // 数字・文字列の削除
    DeleteChars {
        from: Direction,
        offset: usize,
        count: usize,
    },
    DeleteBefore {
        from: Direction,
        n: usize,
    },
    DeletePattern {
        pattern: DeletePattern,
    },
    Make83,
    // 文字種の変換
    CaseConvert {
        conversion: CaseConversion,
        skip_ext: bool,
    },
    KanaConvert {
        conversion: KanaConversion,
        skip_ext: bool,
    },
    WidthConvert {
        conversion: WidthConversion,
        skip_ext: bool,
    },
    ClearDiacritics {
        skip_ext: bool,
    },
    RemoveVoicedMark {
        skip_ext: bool,
    },
    // 文字列の置換
    StringReplace {
        search: String,
        replace: String,
    },
    // 数値の整理
    NumberPad {
        from: Direction,
        n: usize,
        digits: usize,
    },
    NumberAdjust {
        from: Direction,
        n: usize,
        delta: i64,
    },
    // 拡張子の変換
    ExtConvert {
        conversion: ExtConversion,
    },
    ExtDelete,
    ExtAdd {
        ext: String,
    },
    ExtReplace {
        ext: String,
    },
}

// ── ヘルパー ─────────────────────────────────────────────────────

fn split_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) => (&name[..i], &name[i..]),
        None => (name, ""),
    }
}

fn normalize_ext_arg(ext: &str) -> String {
    let trimmed = ext.trim();
    if trimmed.is_empty() {
        String::new()
    } else if let Some(stripped) = trimmed.strip_prefix('.') {
        format!(".{}", stripped)
    } else {
        format!(".{}", trimmed)
    }
}

fn capitalize_words(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut at_word_start = true;
    for c in s.chars() {
        if c.is_whitespace() || c == '_' || c == '-' || c == '.' {
            at_word_start = true;
            out.push(c);
        } else if at_word_start {
            for u in c.to_uppercase() {
                out.push(u);
            }
            at_word_start = false;
        } else {
            for l in c.to_lowercase() {
                out.push(l);
            }
        }
    }
    out
}

fn apply_skip_ext_if<F: Fn(&str) -> String>(name: &str, skip_ext: bool, f: F) -> String {
    if skip_ext {
        convert::apply_skip_ext(name, f)
    } else {
        f(name)
    }
}

fn delete_chars(name: &str, from: Direction, offset: usize, count: usize) -> String {
    let chars: Vec<char> = name.chars().collect();
    let len = chars.len();
    if count == 0 || len == 0 {
        return name.into();
    }
    let (start, end) = match from {
        Direction::Start => (offset.min(len), (offset + count).min(len)),
        Direction::End => {
            let end = len.saturating_sub(offset);
            let start = end.saturating_sub(count);
            (start, end)
        }
    };
    let mut out: String = chars[..start].iter().collect();
    out.extend(chars[end..].iter());
    out
}

fn delete_before(name: &str, from: Direction, n: usize) -> String {
    let chars: Vec<char> = name.chars().collect();
    let len = chars.len();
    match from {
        Direction::Start => chars.iter().skip(n.min(len)).collect(),
        Direction::End => {
            let keep_from = len.saturating_sub(n);
            chars[..keep_from].iter().collect()
        }
    }
}

fn make_83(name: &str) -> String {
    let (stem, ext) = split_ext(name);
    let stem_short: String = stem.chars().take(8).collect();
    let ext_short: String = if ext.is_empty() {
        String::new()
    } else {
        // ext は先頭の "." を含む
        let after_dot: String = ext.chars().skip(1).take(3).collect();
        if after_dot.is_empty() {
            String::new()
        } else {
            format!(".{}", after_dot)
        }
    };
    format!("{}{}", stem_short, ext_short)
}

fn delete_by_pattern(name: &str, pattern: DeletePattern) -> String {
    use DeletePattern::*;
    // 各パターンを regex で検出して取り除く。空白の整理は行わない（必要なら後続ステップ）。
    let pat = match pattern {
        // Windows の「コピー (N)」「 - コピー (N)」「 - Copy」「 - Copy (N)」「コピー」「 - コピー」を網羅
        CopyNum => {
            r"(?:\s*-\s*(?:Copy|コピー)(?:\s*\(\d+\))?|\s*コピー\s*\(\d+\))"
        }
        // Vista 系の「ーコピー」「ーコピー (N)」「ーCopy」「ーCopy (N)」（長音記号区切り）
        CopyNumVista => {
            r"(?:ー(?:Copy|コピー)(?:\s*\(\d+\))?)"
        }
        // 「 - Shortcut」「 - ショートカット」「へのショートカット」
        Shortcut => r"(?:\s*-\s*(?:Shortcut|ショートカット)|\s*へのショートカット)",
        // 「ーShortcut」「ーショートカット」
        ShortcutVista => r"(?:ー(?:Shortcut|ショートカット))",
        // 任意の括弧とその中身（半角・全角・各種日本語括弧）
        BracketContent => {
            r"(?:\[[^\]]*\]|\([^\)]*\)|（[^）]*）|「[^」]*」|『[^』]*』|【[^】]*】|〔[^〕]*〕|《[^》]*》)"
        }
        NumKagiOrRound => r"(?:【\d+】|（\d+）)",
        NumKagi => r"【\d+】",
        NumRound => r"（\d+）",
        NumLenticular => r"〔\d+〕",
    };
    let re = regex::Regex::new(pat).unwrap();
    re.replace_all(name, "").into_owned()
}

fn nth_number_range(name: &str, from: Direction, n: usize) -> Option<(usize, usize)> {
    if n == 0 {
        return None;
    }
    let re = regex::Regex::new(r"\d+").unwrap();
    let matches: Vec<_> = re.find_iter(name).collect();
    if matches.is_empty() {
        return None;
    }
    let idx = match from {
        Direction::Start => n - 1,
        Direction::End => matches.len().checked_sub(n)?,
    };
    matches.get(idx).map(|m| (m.start(), m.end()))
}

fn number_pad(name: &str, from: Direction, n: usize, digits: usize) -> String {
    if let Some((s, e)) = nth_number_range(name, from, n) {
        let num: u64 = name[s..e].parse().unwrap_or(0);
        let padded = format!("{:0>width$}", num, width = digits);
        let mut out = String::with_capacity(name.len() + digits);
        out.push_str(&name[..s]);
        out.push_str(&padded);
        out.push_str(&name[e..]);
        out
    } else {
        name.into()
    }
}

fn number_adjust(name: &str, from: Direction, n: usize, delta: i64) -> String {
    if let Some((s, e)) = nth_number_range(name, from, n) {
        let original_str = &name[s..e];
        let original: i64 = original_str.parse().unwrap_or(0);
        let new_val = original.saturating_add(delta).max(0);
        let width = original_str.len();
        // 元の桁数を維持（先行ゼロは保つ）
        let new_str = format!("{:0>width$}", new_val, width = width);
        let mut out = String::with_capacity(name.len() + width);
        out.push_str(&name[..s]);
        out.push_str(&new_str);
        out.push_str(&name[e..]);
        out
    } else {
        name.into()
    }
}

// ── ディスパッチ ────────────────────────────────────────────────

pub fn apply_builtin(op: &BuiltinOp, current: &str, ctx: &FileContext) -> String {
    use BuiltinOp::*;
    match op {
        // ── 連番・文字列の追加 ──
        AddSeqStr {
            prefix,
            suffix,
            digits,
            start,
            step,
        } => {
            // 仕様: 結果は `{prefix}{seq_padded}{suffix}` でファイル名 stem を置き換え、
            // 拡張子は維持する。op 個別の start/step/digits を優先、numbering は
            // FileContext の global value を継承する（HANDOFF）。
            let (_, ext) = split_ext(current);
            let value = start.saturating_add(ctx.applied_index.saturating_mul(*step));
            let seq = format_seq(value, *digits, ctx.seq_numbering);
            format!("{}{}{}{}", prefix, seq, suffix, ext)
        }
        AddDatetime { format, position } => {
            // `format` は Naire の `\Y\m\d` 系テンプレートとして展開される。
            let formatted = expand_template(format, None, ctx);
            match position {
                Position::Prefix => format!("{}{}", formatted, current),
                Position::Suffix => {
                    let (stem, ext) = split_ext(current);
                    format!("{}{}{}", stem, formatted, ext)
                }
            }
        }
        AddFolderName { position } => match position {
            Position::Prefix => format!("{}{}", ctx.folder_name, current),
            Position::Suffix => {
                let (stem, ext) = split_ext(current);
                format!("{}{}{}", stem, ctx.folder_name, ext)
            }
        },
        AddFolderSeq {
            digits,
            start,
            step,
        } => {
            let (_, ext) = split_ext(current);
            let value = start.saturating_add(ctx.applied_index.saturating_mul(*step));
            let seq = format_seq(value, *digits, ctx.seq_numbering);
            format!("{}{}{}", ctx.folder_name, seq, ext)
        }
        TruncateFromStart { n } => {
            // ファイル名先頭から n 文字を削除（拡張子含む全体に対して）
            delete_before(current, Direction::Start, *n)
        }
        TruncateFromEnd { n } => {
            // ファイル名末尾から n 文字を削除
            delete_before(current, Direction::End, *n)
        }

        // ── 数字・文字列の削除 ──
        DeleteChars { from, offset, count } => delete_chars(current, *from, *offset, *count),
        DeleteBefore { from, n } => delete_before(current, *from, *n),
        DeletePattern { pattern } => delete_by_pattern(current, *pattern),
        Make83 => make_83(current),

        // ── 文字種の変換 ──
        CaseConvert { conversion, skip_ext } => apply_skip_ext_if(current, *skip_ext, |s| {
            match conversion {
                CaseConversion::Capitalize => capitalize_words(s),
                CaseConversion::Upper => s.to_uppercase(),
                CaseConversion::Lower => s.to_lowercase(),
            }
        }),
        KanaConvert { conversion, skip_ext } => apply_skip_ext_if(current, *skip_ext, |s| {
            match conversion {
                KanaConversion::HiraToKata => convert::hiragana_to_katakana(s),
                KanaConversion::KataToHira => convert::katakana_to_hiragana(s),
                KanaConversion::HankakuKataToZenkaku => {
                    convert::hankaku_kata_to_zenkaku(s)
                }
                KanaConversion::ZenkakuKataToHankaku => {
                    convert::zenkaku_kata_to_hankaku(s)
                }
            }
        }),
        WidthConvert { conversion, skip_ext } => apply_skip_ext_if(current, *skip_ext, |s| {
            match conversion {
                WidthConversion::ToZenkaku => convert::hankaku_to_zenkaku_alnum(s),
                WidthConversion::ToHankaku => convert::zenkaku_to_hankaku_alnum(s),
            }
        }),
        ClearDiacritics { skip_ext } => {
            apply_skip_ext_if(current, *skip_ext, convert::clear_diacritics)
        }
        RemoveVoicedMark { skip_ext } => {
            apply_skip_ext_if(current, *skip_ext, convert::remove_voiced_marks)
        }

        // ── 文字列の置換 ──
        StringReplace { search, replace } => {
            if search.is_empty() {
                current.into()
            } else {
                current.replace(search.as_str(), replace.as_str())
            }
        }

        // ── 数値の整理 ──
        NumberPad { from, n, digits } => number_pad(current, *from, *n, *digits),
        NumberAdjust { from, n, delta } => number_adjust(current, *from, *n, *delta),

        // ── 拡張子の変換 ──
        ExtConvert { conversion } => {
            let (stem, ext) = split_ext(current);
            if ext.is_empty() {
                return current.into();
            }
            let new_ext = match conversion {
                ExtConversion::Upper => ext.to_uppercase(),
                ExtConversion::Lower => ext.to_lowercase(),
            };
            format!("{}{}", stem, new_ext)
        }
        ExtDelete => {
            let (stem, _) = split_ext(current);
            stem.into()
        }
        ExtAdd { ext } => {
            let normalized = normalize_ext_arg(ext);
            format!("{}{}", current, normalized)
        }
        ExtReplace { ext } => {
            let (stem, _) = split_ext(current);
            let normalized = normalize_ext_arg(ext);
            format!("{}{}", stem, normalized)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn ctx(path: &str) -> FileContext {
        FileContext::from_path(&PathBuf::from(path))
    }

    #[test]
    fn add_seq_str_replaces_stem() {
        let mut c = ctx("/x/folder/photo.jpg");
        c.applied_index = 0;
        let op = BuiltinOp::AddSeqStr {
            prefix: "img_".into(),
            suffix: "".into(),
            digits: 3,
            start: 0,
            step: 1,
        };
        assert_eq!(apply_builtin(&op, "photo.jpg", &c), "img_000.jpg");
        let mut c2 = c;
        c2.applied_index = 5;
        assert_eq!(apply_builtin(&op, "photo.jpg", &c2), "img_005.jpg");
    }

    #[test]
    fn add_folder_name_prefix() {
        let c = ctx("/x/folder/file.txt");
        let op = BuiltinOp::AddFolderName {
            position: Position::Prefix,
        };
        assert_eq!(apply_builtin(&op, "file.txt", &c), "folderfile.txt");
    }

    #[test]
    fn add_folder_name_suffix() {
        let c = ctx("/x/folder/file.txt");
        let op = BuiltinOp::AddFolderName {
            position: Position::Suffix,
        };
        assert_eq!(apply_builtin(&op, "file.txt", &c), "filefolder.txt");
    }

    #[test]
    fn truncate_from_start_and_end() {
        let c = ctx("/x/folder/abcdefgh.txt");
        let op_s = BuiltinOp::TruncateFromStart { n: 3 };
        assert_eq!(apply_builtin(&op_s, "abcdefgh.txt", &c), "defgh.txt");
        let op_e = BuiltinOp::TruncateFromEnd { n: 4 };
        assert_eq!(apply_builtin(&op_e, "abcdefgh.txt", &c), "abcdefgh");
    }

    #[test]
    fn delete_chars_from_start() {
        let c = ctx("/x/file.txt");
        let op = BuiltinOp::DeleteChars {
            from: Direction::Start,
            offset: 2,
            count: 3,
        };
        assert_eq!(apply_builtin(&op, "abcdefgh", &c), "abfgh");
    }

    #[test]
    fn delete_chars_from_end() {
        let c = ctx("/x/file.txt");
        let op = BuiltinOp::DeleteChars {
            from: Direction::End,
            offset: 1,
            count: 3,
        };
        assert_eq!(apply_builtin(&op, "abcdefgh", &c), "abcdh");
    }

    #[test]
    fn delete_pattern_copy_num() {
        let c = ctx("/x/file.txt");
        let op = BuiltinOp::DeletePattern {
            pattern: DeletePattern::CopyNum,
        };
        assert_eq!(apply_builtin(&op, "file - Copy.txt", &c), "file.txt");
        assert_eq!(
            apply_builtin(&op, "file - Copy (3).txt", &c),
            "file.txt"
        );
        assert_eq!(
            apply_builtin(&op, "file - コピー (2).txt", &c),
            "file.txt"
        );
    }

    #[test]
    fn delete_pattern_brackets() {
        let c = ctx("/x/file.txt");
        let op = BuiltinOp::DeletePattern {
            pattern: DeletePattern::BracketContent,
        };
        // 注: 括弧周辺の空白は維持される（必要なら後続ステップで整形）
        assert_eq!(
            apply_builtin(&op, "[meta]file(v2)【NEW】.txt", &c),
            "file.txt"
        );
    }

    #[test]
    fn delete_pattern_num_kagi() {
        let c = ctx("/x/file.txt");
        let op = BuiltinOp::DeletePattern {
            pattern: DeletePattern::NumKagi,
        };
        assert_eq!(apply_builtin(&op, "title【01】.txt", &c), "title.txt");
    }

    #[test]
    fn make_83_truncates() {
        let c = ctx("/x/file.txt");
        let op = BuiltinOp::Make83;
        assert_eq!(
            apply_builtin(&op, "verylongfilename.docx", &c),
            "verylong.doc"
        );
        assert_eq!(apply_builtin(&op, "noext", &c), "noext");
    }

    #[test]
    fn case_convert_upper_skip_ext() {
        let c = ctx("/x/file.txt");
        let op = BuiltinOp::CaseConvert {
            conversion: CaseConversion::Upper,
            skip_ext: true,
        };
        assert_eq!(apply_builtin(&op, "hello.txt", &c), "HELLO.txt");
        let op2 = BuiltinOp::CaseConvert {
            conversion: CaseConversion::Upper,
            skip_ext: false,
        };
        assert_eq!(apply_builtin(&op2, "hello.txt", &c), "HELLO.TXT");
    }

    #[test]
    fn case_convert_capitalize() {
        let c = ctx("/x/file.txt");
        let op = BuiltinOp::CaseConvert {
            conversion: CaseConversion::Capitalize,
            skip_ext: true,
        };
        assert_eq!(apply_builtin(&op, "hello world.txt", &c), "Hello World.txt");
    }

    #[test]
    fn kana_hira_to_kata() {
        let c = ctx("/x/file.txt");
        let op = BuiltinOp::KanaConvert {
            conversion: KanaConversion::HiraToKata,
            skip_ext: true,
        };
        assert_eq!(apply_builtin(&op, "ひらがな.txt", &c), "ヒラガナ.txt");
    }

    #[test]
    fn remove_voiced_mark() {
        let c = ctx("/x/file.txt");
        let op = BuiltinOp::RemoveVoicedMark { skip_ext: true };
        assert_eq!(apply_builtin(&op, "ガタタン.txt", &c), "カタタン.txt");
    }

    #[test]
    fn string_replace_global() {
        let c = ctx("/x/file.txt");
        let op = BuiltinOp::StringReplace {
            search: "foo".into(),
            replace: "bar".into(),
        };
        assert_eq!(apply_builtin(&op, "foofoofoo.txt", &c), "barbarbar.txt");
    }

    #[test]
    fn number_pad_first() {
        let c = ctx("/x/file.txt");
        let op = BuiltinOp::NumberPad {
            from: Direction::Start,
            n: 1,
            digits: 3,
        };
        assert_eq!(apply_builtin(&op, "img_5.jpg", &c), "img_005.jpg");
    }

    #[test]
    fn number_adjust_subtract() {
        let c = ctx("/x/file.txt");
        let op = BuiltinOp::NumberAdjust {
            from: Direction::End,
            n: 1,
            delta: -2,
        };
        assert_eq!(apply_builtin(&op, "page_05.txt", &c), "page_03.txt");
    }

    #[test]
    fn ext_convert_upper() {
        let c = ctx("/x/file.txt");
        let op = BuiltinOp::ExtConvert {
            conversion: ExtConversion::Upper,
        };
        assert_eq!(apply_builtin(&op, "file.txt", &c), "file.TXT");
    }

    #[test]
    fn ext_delete() {
        let c = ctx("/x/file.txt");
        let op = BuiltinOp::ExtDelete;
        assert_eq!(apply_builtin(&op, "file.txt", &c), "file");
        assert_eq!(apply_builtin(&op, "no_ext", &c), "no_ext");
    }

    #[test]
    fn ext_add_normalizes_dot() {
        let c = ctx("/x/file");
        let op_with = BuiltinOp::ExtAdd { ext: ".bak".into() };
        assert_eq!(apply_builtin(&op_with, "file", &c), "file.bak");
        let op_without = BuiltinOp::ExtAdd { ext: "bak".into() };
        assert_eq!(apply_builtin(&op_without, "file", &c), "file.bak");
    }

    #[test]
    fn ext_replace() {
        let c = ctx("/x/file.txt");
        let op = BuiltinOp::ExtReplace { ext: "md".into() };
        assert_eq!(apply_builtin(&op, "file.txt", &c), "file.md");
    }
}
