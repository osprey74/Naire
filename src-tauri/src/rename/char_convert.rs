//! 文字変換モード — `tr(1)` 風の 1:1 文字（または複数文字トークン）変換。
//!
//! 入力構文（`from` / `to` どちらも共通）:
//! - **列挙**: 文字列にカンマ `,` を含む場合、カンマで区切られた各トークンが
//!   1 要素になる（例: `0,i,ii,iii,iv,v,vi,vii,viii,ix,x`）。マルチ文字トークン
//!   が必要なときに使う。
//! - **範囲**: それ以外は文字単位でスキャンし、`X-Y` パターンを見つけたら
//!   `X` から `Y` までの全文字を 1 要素ずつ展開する（例: `a-z` → 26 要素）。
//!   範囲でないものは 1 文字 1 要素として扱う。
//!
//! 変換時の挙動:
//! - 入力文字列をスキャンし、`from` の各要素を **長いものから優先** マッチ。
//! - 一致した部分を対応する `to` 要素で置換。
//! - `to` が `from` より短いときは末尾要素でパディング、空のときは削除。

/// 文字セット文字列を要素列にパースする。
pub fn parse_charset(s: &str) -> Vec<String> {
    if s.contains(',') {
        return s.split(',').map(|t| t.to_string()).collect();
    }
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if i + 2 < chars.len() && chars[i + 1] == '-' {
            let from_ch = chars[i];
            let to_ch = chars[i + 2];
            let (lo, hi) = (from_ch as u32, to_ch as u32);
            if lo <= hi {
                for cp in lo..=hi {
                    if let Some(c) = char::from_u32(cp) {
                        out.push(c.to_string());
                    }
                }
                i += 3;
                continue;
            }
        }
        out.push(chars[i].to_string());
        i += 1;
    }
    out
}

/// `from` と `to` から (検索文字列, 置換文字列) のペア列を作る。
/// マルチ文字トークンの貪欲マッチを成立させるため、検索文字列の長さ降順で
/// ソートする。要素数の差異は HANDOFF 流の `tr` 挙動で吸収:
/// - `to` が短い: 末尾要素でパディング
/// - `to` が空: 入力から削除（空文字列にマップ）
pub fn build_mapping(from: &str, to: &str) -> Vec<(String, String)> {
    let from_set = parse_charset(from);
    let to_set = parse_charset(to);
    let mut pairs: Vec<(String, String)> = from_set
        .into_iter()
        .enumerate()
        .filter(|(_, f)| !f.is_empty())
        .map(|(i, f)| {
            let t = if to_set.is_empty() {
                String::new()
            } else if i < to_set.len() {
                to_set[i].clone()
            } else {
                to_set.last().cloned().unwrap_or_default()
            };
            (f, t)
        })
        .collect();
    pairs.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    pairs
}

/// 入力に対して mapping を貪欲適用する。
pub fn apply(input: &str, mapping: &[(String, String)]) -> String {
    let mut out = String::with_capacity(input.len());
    let mut pos = 0;
    while pos < input.len() {
        let rest = &input[pos..];
        let mut matched = false;
        for (f, t) in mapping {
            if rest.starts_with(f.as_str()) {
                out.push_str(t);
                pos += f.len();
                matched = true;
                break;
            }
        }
        if !matched {
            let c = rest.chars().next().unwrap();
            out.push(c);
            pos += c.len_utf8();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_range_ascii() {
        let v = parse_charset("a-e");
        assert_eq!(v, vec!["a", "b", "c", "d", "e"]);
    }

    #[test]
    fn parse_range_zenkaku_digits() {
        let v = parse_charset("０-９");
        assert_eq!(v.len(), 10);
        assert_eq!(v[0], "０");
        assert_eq!(v[9], "９");
    }

    #[test]
    fn parse_enum_comma() {
        let v = parse_charset("0,i,ii,iii");
        assert_eq!(v, vec!["0", "i", "ii", "iii"]);
    }

    #[test]
    fn parse_literal_chars_no_range() {
        let v = parse_charset("〇一二三");
        assert_eq!(v, vec!["〇", "一", "二", "三"]);
    }

    #[test]
    fn apply_basic_lowercase_to_upper() {
        let m = build_mapping("a-z", "A-Z");
        assert_eq!(apply("hello", &m), "HELLO");
    }

    #[test]
    fn apply_zenkaku_digits_to_hankaku() {
        let m = build_mapping("０-９", "0-9");
        assert_eq!(apply("１２３", &m), "123");
    }

    #[test]
    fn apply_roman_numerals_greedy() {
        let m = build_mapping(
            "0,i,ii,iii,iv,v,vi,vii,viii,ix,x",
            "0,1,2,3,4,5,6,7,8,9,10",
        );
        // "viii" は "v"+"iii" や "vi"+"ii" ではなく "viii" として一致するはず
        assert_eq!(apply("viii", &m), "8");
        assert_eq!(apply("ix-iv", &m), "9-4");
    }

    #[test]
    fn apply_unmapped_chars_pass_through() {
        let m = build_mapping("a-z", "A-Z");
        assert_eq!(apply("abc123XYZ", &m), "ABC123XYZ");
    }

    #[test]
    fn apply_empty_to_deletes() {
        let m = build_mapping("aeiou", "");
        assert_eq!(apply("hello world", &m), "hll wrld");
    }

    #[test]
    fn apply_shorter_to_pads_last() {
        // a-e → "X" → 全部 X にマップ
        let m = build_mapping("a-e", "X");
        assert_eq!(apply("abcde", &m), "XXXXX");
    }

    #[test]
    fn apply_kanji_to_arabic() {
        let m = build_mapping("〇一二三四五六七八九十", "0-9X");
        // 漢数字 11 個 vs 0-9 10 要素 + X 1 要素 = 11 要素
        assert_eq!(apply("一二三", &m), "123");
        assert_eq!(apply("十", &m), "X");
    }
}
