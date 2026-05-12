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
        // alpha は bijective なので桁固定の概念を持たず、`digits` は無視する。
        Numbering::Alpha => format_alpha(value),
    }
}

/// 英大文字連番（bijective base-26、0 始まり）。
///
/// A=0, B=1, ..., Z=25, AA=26, AB=27, ..., AZ=51, BA=52, ..., ZZ=701, AAA=702。
/// 文字数は値に応じて自然に伸びるため、`digits` パラメータによるパディングは
/// 行わない（positional 系のように値域途中で桁が増える違和感を避ける）。
fn format_alpha(value: u64) -> String {
    let mut n = value + 1; // 1-indexed の bijective に変換
    let mut chars = Vec::new();
    while n > 0 {
        n -= 1;
        chars.push((b'A' + (n % 26) as u8) as char);
        n /= 26;
    }
    chars.iter().rev().collect()
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
    fn alpha_bijective_single_digit() {
        assert_eq!(format_seq(0, 1, Numbering::Alpha), "A");
        assert_eq!(format_seq(25, 1, Numbering::Alpha), "Z");
    }

    #[test]
    fn alpha_bijective_two_digit() {
        // HANDOFF マッピング表どおりの bijective base-26
        assert_eq!(format_seq(26, 1, Numbering::Alpha), "AA");
        assert_eq!(format_seq(51, 1, Numbering::Alpha), "AZ");
        assert_eq!(format_seq(52, 1, Numbering::Alpha), "BA");
        assert_eq!(format_seq(701, 1, Numbering::Alpha), "ZZ");
    }

    #[test]
    fn alpha_bijective_three_digit() {
        assert_eq!(format_seq(702, 1, Numbering::Alpha), "AAA");
    }

    #[test]
    fn alpha_ignores_digits_param() {
        // digits を指定してもパディングしない（bijective の挙動）
        assert_eq!(format_seq(0, 3, Numbering::Alpha), "A");
        assert_eq!(format_seq(25, 4, Numbering::Alpha), "Z");
        assert_eq!(format_seq(26, 4, Numbering::Alpha), "AA");
    }

    #[test]
    fn parse_numbering() {
        assert_eq!(Numbering::parse("decimal"), Numbering::Decimal);
        assert_eq!(Numbering::parse("hex"), Numbering::Hex);
        assert_eq!(Numbering::parse("alpha"), Numbering::Alpha);
        assert_eq!(Numbering::parse("unknown"), Numbering::Decimal);
    }
}
