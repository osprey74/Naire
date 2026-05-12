use regex::Regex;

use crate::rename::variables::{expand_template, FileContext};

/// 単一の名前に正規表現ステップを適用する。マッチしなければ入力をそのまま返す。
pub fn apply_regex(re: &Regex, replace_template: &str, current: &str, ctx: &FileContext) -> String {
    re.replace_all(current, |caps: &regex::Captures| {
        expand_template(replace_template, Some(caps), ctx)
    })
    .into_owned()
}

/// ワイルドカードパターンを正規表現パターンに変換する。
///
/// 変換規則（HANDOFF）:
/// - `?` → `.` （任意の 1 文字）
/// - `*` → `.*`（任意の文字列）
/// - その他の正規表現メタ文字（`.` `(` `)` `[` `]` `{` `}` `+` `^` `$` `|` `\`）は
///   `\X` でエスケープ
///
/// パターン全体を `^...$` でアンカーしてファイル名全体マッチを要求する。
/// ワイルドカードはキャプチャグループを生成しないため、replace 側で `\1`〜`\9` は
/// 参照できない。マッチ全体を参照したい場合は `\0`（元のファイル名）、
/// ステム部分なら `\t` を使う。
pub fn wildcard_to_regex(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len() + 4);
    out.push('^');
    for c in pattern.chars() {
        match c {
            '?' => out.push('.'),
            '*' => out.push_str(".*"),
            '.' | '\\' | '(' | ')' | '[' | ']' | '{' | '}' | '+' | '^' | '$' | '|' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out.push('$');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn ctx(path: &str) -> FileContext {
        FileContext::from_path(&PathBuf::from(path))
    }

    #[test]
    fn replace_with_capture() {
        let re = Regex::new(r"^(\d+)_(.+)\.txt$").unwrap();
        let c = ctx("/x/folder/01_alpha.txt");
        assert_eq!(apply_regex(&re, r"\2-\1.txt", "01_alpha.txt", &c), "alpha-01.txt");
    }

    #[test]
    fn replace_with_file_vars() {
        let re = Regex::new(r"^(.+)$").unwrap();
        let c = ctx("/x/folder/file.txt");
        assert_eq!(apply_regex(&re, r"\f_\1", "file.txt", &c), "folder_file.txt");
    }

    #[test]
    fn no_match_returns_original() {
        let re = Regex::new(r"^xyz$").unwrap();
        let c = ctx("/x/folder/file.txt");
        assert_eq!(apply_regex(&re, "replaced", "file.txt", &c), "file.txt");
    }

    #[test]
    fn wildcard_basic_translation() {
        assert_eq!(wildcard_to_regex("*.jpg"), r"^.*\.jpg$");
        assert_eq!(wildcard_to_regex("file?.txt"), r"^file.\.txt$");
        assert_eq!(wildcard_to_regex("*"), "^.*$");
    }

    #[test]
    fn wildcard_escapes_meta() {
        assert_eq!(wildcard_to_regex("a+b"), r"^a\+b$");
        assert_eq!(wildcard_to_regex("[meta]"), r"^\[meta\]$");
        assert_eq!(wildcard_to_regex("a|b"), r"^a\|b$");
    }

    #[test]
    fn wildcard_anchors_full_filename() {
        let re = Regex::new(&wildcard_to_regex("*.txt")).unwrap();
        assert!(re.is_match("hello.txt"));
        assert!(re.is_match(".txt"));
        // 末尾が .txt でないものは不一致
        assert!(!re.is_match("hello.txt.bak"));
    }
}
