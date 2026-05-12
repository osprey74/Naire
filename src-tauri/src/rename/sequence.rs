#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Numbering {
    Decimal,
    Hex,
    Alpha,
}

impl Numbering {
    pub fn parse(s: &str) -> Self {
        match s {
            "hex" => Numbering::Hex,
            "alpha" => Numbering::Alpha,
            _ => Numbering::Decimal,
        }
    }
}

pub fn format_seq(value: u64, digits: usize, numbering: Numbering) -> String {
    let digits = digits.max(1);
    match numbering {
        Numbering::Decimal => format!("{:0>width$}", value, width = digits),
        Numbering::Hex => format!("{:0>width$X}", value, width = digits),
        Numbering::Alpha => format_alpha(value, digits),
    }
}

/// 英大文字連番（A=0, B=1, ..., Z=25, BA=26, ZZ=675, BAA=676, ...）。
///
/// **NOTE**: HANDOFF の `alpha` 仕様は表記揺れがあり、マッピング表
/// （`ZZ=701, AAA=702`）は bijective base-26 を示しているが、例示列
/// （`AA, AB, ..., AZ, BA, ...`）は positional base-26 を示している。
/// bijective だと、`digits=2` で値 0→"AA"(pad), 26→"AA"(natural) が
/// 同一文字列を生成する重複が発生してリネーム衝突を引き起こすため、
/// Naire では **positional base-26 with A=0 padding** を採用した。
/// 結果として `ZZ=675, BAA=676` となり、マッピング表とはずれる。
fn format_alpha(value: u64, digits: usize) -> String {
    let mut chars = Vec::new();
    if value == 0 {
        chars.push('A');
    } else {
        let mut v = value;
        while v > 0 {
            chars.push((b'A' + (v % 26) as u8) as char);
            v /= 26;
        }
    }
    let mut s: String = chars.iter().rev().collect();
    while s.len() < digits {
        s.insert(0, 'A');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal() {
        assert_eq!(format_seq(0, 2, Numbering::Decimal), "00");
        assert_eq!(format_seq(5, 2, Numbering::Decimal), "05");
        assert_eq!(format_seq(100, 2, Numbering::Decimal), "100");
    }

    #[test]
    fn hex() {
        assert_eq!(format_seq(15, 2, Numbering::Hex), "0F");
        assert_eq!(format_seq(255, 2, Numbering::Hex), "FF");
        assert_eq!(format_seq(256, 2, Numbering::Hex), "100");
    }

    #[test]
    fn alpha_padding() {
        assert_eq!(format_seq(0, 2, Numbering::Alpha), "AA");
        assert_eq!(format_seq(25, 2, Numbering::Alpha), "AZ");
    }

    #[test]
    fn alpha_positional() {
        // 例示列に従う positional base-26
        assert_eq!(format_seq(26, 2, Numbering::Alpha), "BA");
        assert_eq!(format_seq(51, 2, Numbering::Alpha), "BZ");
        assert_eq!(format_seq(675, 2, Numbering::Alpha), "ZZ");
        // 桁あふれ
        assert_eq!(format_seq(676, 2, Numbering::Alpha), "BAA");
    }

    #[test]
    fn alpha_three_digit_padding() {
        assert_eq!(format_seq(0, 3, Numbering::Alpha), "AAA");
        assert_eq!(format_seq(25, 3, Numbering::Alpha), "AAZ");
        assert_eq!(format_seq(26, 3, Numbering::Alpha), "ABA");
    }

    #[test]
    fn parse_numbering() {
        assert_eq!(Numbering::parse("decimal"), Numbering::Decimal);
        assert_eq!(Numbering::parse("hex"), Numbering::Hex);
        assert_eq!(Numbering::parse("alpha"), Numbering::Alpha);
        assert_eq!(Numbering::parse("unknown"), Numbering::Decimal);
    }
}
