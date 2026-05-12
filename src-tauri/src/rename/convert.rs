use unicode_normalization::UnicodeNormalization;

/// ひらがな → カタカナ（U+3041–U+3096 → U+30A1–U+30F6）
pub fn hiragana_to_katakana(s: &str) -> String {
    s.chars()
        .map(|c| {
            if ('\u{3041}'..='\u{3096}').contains(&c) {
                char::from_u32(c as u32 + 0x60).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

/// カタカナ → ひらがな（U+30A1–U+30F6 → U+3041–U+3096）
pub fn katakana_to_hiragana(s: &str) -> String {
    s.chars()
        .map(|c| {
            if ('\u{30A1}'..='\u{30F6}').contains(&c) {
                char::from_u32(c as u32 - 0x60).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

/// 全角英数記号 → 半角（U+FF01–U+FF5E → U+0021–U+007E）
pub fn zenkaku_to_hankaku_alnum(s: &str) -> String {
    s.chars()
        .map(|c| {
            if ('\u{FF01}'..='\u{FF5E}').contains(&c) {
                char::from_u32(c as u32 - 0xFEE0).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

/// 半角英数記号 → 全角（U+0021–U+007E → U+FF01–U+FF5E）
pub fn hankaku_to_zenkaku_alnum(s: &str) -> String {
    s.chars()
        .map(|c| {
            if ('\u{0021}'..='\u{007E}').contains(&c) {
                char::from_u32(c as u32 + 0xFEE0).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

/// 濁音・半濁音除去（ひらがな・カタカナ共通）
/// NFD で結合濁点 U+3099 / 結合半濁点 U+309A を分解 → 除去 → NFC 再合成。
pub fn remove_voiced_marks(s: &str) -> String {
    s.nfd()
        .filter(|&c| c != '\u{3099}' && c != '\u{309A}')
        .collect::<String>()
        .nfc()
        .collect()
}

/// ダイアクリティカルマーク除去（ラテン文字向け）
/// NFD → Mn（Non-spacing Mark）カテゴリを除去 → NFC
pub fn clear_diacritics(s: &str) -> String {
    s.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect::<String>()
        .nfc()
        .collect()
}

/// 半角カタカナ ↔ 全角カタカナのマッピング表（基底字のみ）。
/// 濁点・半濁点は別途 NFD/NFC で合成・分解する。
const HANKAKU_TO_ZENKAKU: &[(char, char)] = &[
    ('｡', '。'), ('｢', '「'), ('｣', '」'), ('､', '、'), ('･', '・'),
    ('ｦ', 'ヲ'),
    ('ｧ', 'ァ'), ('ｨ', 'ィ'), ('ｩ', 'ゥ'), ('ｪ', 'ェ'), ('ｫ', 'ォ'),
    ('ｬ', 'ャ'), ('ｭ', 'ュ'), ('ｮ', 'ョ'),
    ('ｯ', 'ッ'), ('ｰ', 'ー'),
    ('ｱ', 'ア'), ('ｲ', 'イ'), ('ｳ', 'ウ'), ('ｴ', 'エ'), ('ｵ', 'オ'),
    ('ｶ', 'カ'), ('ｷ', 'キ'), ('ｸ', 'ク'), ('ｹ', 'ケ'), ('ｺ', 'コ'),
    ('ｻ', 'サ'), ('ｼ', 'シ'), ('ｽ', 'ス'), ('ｾ', 'セ'), ('ｿ', 'ソ'),
    ('ﾀ', 'タ'), ('ﾁ', 'チ'), ('ﾂ', 'ツ'), ('ﾃ', 'テ'), ('ﾄ', 'ト'),
    ('ﾅ', 'ナ'), ('ﾆ', 'ニ'), ('ﾇ', 'ヌ'), ('ﾈ', 'ネ'), ('ﾉ', 'ノ'),
    ('ﾊ', 'ハ'), ('ﾋ', 'ヒ'), ('ﾌ', 'フ'), ('ﾍ', 'ヘ'), ('ﾎ', 'ホ'),
    ('ﾏ', 'マ'), ('ﾐ', 'ミ'), ('ﾑ', 'ム'), ('ﾒ', 'メ'), ('ﾓ', 'モ'),
    ('ﾔ', 'ヤ'), ('ﾕ', 'ユ'), ('ﾖ', 'ヨ'),
    ('ﾗ', 'ラ'), ('ﾘ', 'リ'), ('ﾙ', 'ル'), ('ﾚ', 'レ'), ('ﾛ', 'ロ'),
    ('ﾜ', 'ワ'), ('ﾝ', 'ン'),
];

fn compose_voiced(base: char, dakuten: bool) -> Option<char> {
    let mark = if dakuten { '\u{3099}' } else { '\u{309A}' };
    let s: String = [base, mark].iter().collect();
    let composed: String = s.nfc().collect();
    let mut iter = composed.chars();
    let first = iter.next()?;
    if iter.next().is_none() {
        Some(first)
    } else {
        None
    }
}

/// 半角カタカナ → 全角カタカナ。
/// 濁点 `ﾞ`(U+FF9E) と半濁点 `ﾟ`(U+FF9F) は直前の文字と合成する
/// （例: `ｶﾞ` → `ガ`、`ﾊﾟ` → `パ`）。
pub fn hankaku_kata_to_zenkaku(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let zenkaku = HANKAKU_TO_ZENKAKU
            .iter()
            .find(|(h, _)| *h == c)
            .map(|(_, z)| *z);
        if let Some(z) = zenkaku {
            if i + 1 < chars.len() {
                let next = chars[i + 1];
                if next == 'ﾞ' {
                    if let Some(merged) = compose_voiced(z, true) {
                        out.push(merged);
                        i += 2;
                        continue;
                    }
                } else if next == 'ﾟ' {
                    if let Some(merged) = compose_voiced(z, false) {
                        out.push(merged);
                        i += 2;
                        continue;
                    }
                }
            }
            out.push(z);
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

/// 全角カタカナ → 半角カタカナ。
/// 合成済みの濁音・半濁音は NFD で分解してから半角の `ﾞ` `ﾟ` に変換する。
pub fn zenkaku_kata_to_hankaku(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.nfd() {
        if c == '\u{3099}' {
            out.push('ﾞ');
        } else if c == '\u{309A}' {
            out.push('ﾟ');
        } else if let Some((h, _)) = HANKAKU_TO_ZENKAKU.iter().find(|(_, z)| *z == c) {
            out.push(*h);
        } else {
            out.push(c);
        }
    }
    out
}

/// 拡張子を除いた部分にのみ変換を適用するラッパー
pub fn apply_skip_ext<F: Fn(&str) -> String>(filename: &str, f: F) -> String {
    match filename.rfind('.') {
        Some(dot) => format!("{}{}", f(&filename[..dot]), &filename[dot..]),
        None => f(filename),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voiced_mark_removal() {
        assert_eq!(remove_voiced_marks("が"), "か");
        assert_eq!(remove_voiced_marks("ぱ"), "は");
        assert_eq!(remove_voiced_marks("ヴ"), "ウ");
        assert_eq!(remove_voiced_marks("ガタタン"), "カタタン");
    }

    #[test]
    fn kana_conversion() {
        assert_eq!(hiragana_to_katakana("あいう"), "アイウ");
        assert_eq!(katakana_to_hiragana("アイウ"), "あいう");
    }

    #[test]
    fn width_conversion() {
        assert_eq!(zenkaku_to_hankaku_alnum("ＡＢＣ１２３"), "ABC123");
        assert_eq!(hankaku_to_zenkaku_alnum("ABC123"), "ＡＢＣ１２３");
    }

    #[test]
    fn hankaku_kata_basic() {
        assert_eq!(hankaku_kata_to_zenkaku("ｱｲｳ"), "アイウ");
        assert_eq!(hankaku_kata_to_zenkaku("ｶﾞｷﾞｸﾞ"), "ガギグ");
        assert_eq!(hankaku_kata_to_zenkaku("ﾊﾟﾋﾟﾌﾟ"), "パピプ");
    }

    #[test]
    fn zenkaku_kata_basic() {
        assert_eq!(zenkaku_kata_to_hankaku("アイウ"), "ｱｲｳ");
        assert_eq!(zenkaku_kata_to_hankaku("ガギグ"), "ｶﾞｷﾞｸﾞ");
        assert_eq!(zenkaku_kata_to_hankaku("パピプ"), "ﾊﾟﾋﾟﾌﾟ");
    }
}
