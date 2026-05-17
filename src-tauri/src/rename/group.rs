//! フォルダ集約機能（Phase 14）
//!
//! 複数フォルダを共通プレフィックスから生成した親フォルダに集約する。
//! Naire の「リネームのみ」原則の唯一の例外。
//!
//! 詳細仕様は HANDOFF_naire.md の「## フォルダ集約仕様」を参照。

use std::path::PathBuf;

use crate::rename::sequence::{format_seq, Numbering};

/// 半角スペース `U+0020` または全角スペース `U+3000` か。
fn is_space(c: char) -> bool {
    c == ' ' || c == '\u{3000}'
}

/// 文字（char）単位で最長共通先頭部分文字列を抽出し、
/// 末尾の半角スペース `U+0020` と全角スペース `U+3000` のみを trim する。
///
/// アルゴリズム:
/// 1. char 単位で共通プレフィックス `P` を算出
/// 2. `P` がいずれかの入力文字列の完全 prefix（char 数一致）であれば、末尾スペースのみ trim して返す
/// 3. それ以外（途中で切れている）の場合、`P` 内の最後の空白位置で切り戻し、末尾スペース trim
/// 4. 空白が一切存在しない場合は、末尾スペース trim だけ行った `P` をそのまま fallback
///
/// バイト境界ではなく Unicode char 単位で比較するため、日本語ファイル名でも安全。
/// 括弧類・記号は trim しない（UI で編集可能）。
pub fn longest_common_prefix(names: &[&str]) -> String {
    if names.is_empty() {
        return String::new();
    }

    let first = names[0];
    let mut max_chars = first.chars().count();
    for name in &names[1..] {
        let common = first
            .chars()
            .zip(name.chars())
            .take_while(|(a, b)| a == b)
            .count();
        if common < max_chars {
            max_chars = common;
        }
    }

    if max_chars == 0 {
        return String::new();
    }

    let prefix: String = first.chars().take(max_chars).collect();

    // P がいずれかの入力の完全 prefix（char 数一致）か？
    let any_complete = names.iter().any(|n| n.chars().count() == max_chars);
    if any_complete {
        return prefix.trim_end_matches(is_space).to_string();
    }

    // P は途中で切れている → 最後の空白で切り戻す
    if let Some((idx, _)) = prefix
        .char_indices()
        .filter(|(_, c)| is_space(*c))
        .last()
    {
        return prefix[..idx].trim_end_matches(is_space).to_string();
    }

    // 空白なし → 末尾スペース trim だけして fallback
    prefix.trim_end_matches(is_space).to_string()
}

/// 集約後の連番リネーム仕様（定型 #1 `add_seq_str` と同一動作）。
///
/// `digits` / `start` / `step` は op 個別の値を使用、`numbering` は
/// グローバル SequenceConfig から継承する（集約 UI から指定）。
#[derive(Clone, Debug)]
pub struct GroupRenameSpec {
    pub prefix: String,
    pub suffix: String,
    pub digits: usize,
    pub start: u64,
    pub step: u64,
    pub numbering: Numbering,
}

/// 連番リネーム後の単一フォルダの計画。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupItemPlan {
    pub original_name: String,
    pub renamed: String,
    pub original_path: String, // parent_folder/original_name
    pub final_path: String,    // parent_folder/group_name/renamed
}

/// 集約処理の全体計画（ディスク変更前のドライラン結果）。
#[derive(Clone, Debug)]
pub struct GroupPlan {
    pub group_path: String, // parent_folder/group_name
    pub items: Vec<GroupItemPlan>, // フォルダ名昇順
}

/// 1 件分の連番リネーム結果を生成する。
/// index は 0 始まり: value = start + index * step、それを `format_seq` で書式化し
/// `{prefix}{seq}{suffix}` で結合する。
pub fn format_group_rename(spec: &GroupRenameSpec, index: u64) -> String {
    let value = spec.start.saturating_add(index.saturating_mul(spec.step));
    let seq = format_seq(value, spec.digits, spec.numbering);
    format!("{}{}{}", spec.prefix, seq, spec.suffix)
}

/// 集約計画を構築する（ディスクには触らない）。
///
/// - `selected_names` をフォルダ名昇順でソートしてから連番を割り当て
/// - `rename` が `None` の場合は元名をそのまま使う（移動のみ）
/// - 戻り値の `items.len()` は `selected_names.len()` と一致
pub fn build_plan(
    parent_folder: &str,
    selected_names: &[String],
    group_name: &str,
    rename: Option<&GroupRenameSpec>,
) -> GroupPlan {
    let parent = PathBuf::from(parent_folder);
    let group_path = parent.join(group_name);

    // フォルダ名昇順ソート
    let mut sorted: Vec<String> = selected_names.to_vec();
    sorted.sort();

    let items: Vec<GroupItemPlan> = sorted
        .iter()
        .enumerate()
        .map(|(idx, name)| {
            let renamed = match rename {
                Some(spec) => format_group_rename(spec, idx as u64),
                None => name.clone(),
            };
            let original_path = parent.join(name).to_string_lossy().into_owned();
            let final_path = group_path.join(&renamed).to_string_lossy().into_owned();
            GroupItemPlan {
                original_name: name.clone(),
                renamed,
                original_path,
                final_path,
            }
        })
        .collect();

    GroupPlan {
        group_path: group_path.to_string_lossy().into_owned(),
        items,
    }
}

/// 計画内の `renamed` が重複していないかチェック。
/// 重複している場合は最初に見つかった衝突名を Some で返す。
pub fn find_renamed_collision(plan: &GroupPlan) -> Option<String> {
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for item in &plan.items {
        if !seen.insert(item.renamed.as_str()) {
            return Some(item.renamed.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_returns_empty() {
        let names: Vec<&str> = vec![];
        assert_eq!(longest_common_prefix(&names), "");
    }

    #[test]
    fn single_element_returns_itself_trimmed() {
        // 単一要素はそのまま（末尾スペースは trim）
        assert_eq!(longest_common_prefix(&["abc"]), "abc");
        assert_eq!(longest_common_prefix(&["abc   "]), "abc");
        assert_eq!(longest_common_prefix(&["abc\u{3000}\u{3000}"]), "abc");
    }

    #[test]
    fn exact_match_returns_full_string() {
        assert_eq!(longest_common_prefix(&["abc", "abc", "abc"]), "abc");
    }

    #[test]
    fn no_common_prefix_returns_empty() {
        assert_eq!(longest_common_prefix(&["abc", "xyz"]), "");
    }

    #[test]
    fn japanese_mixed_prefix() {
        // 提示された実例：共通部分は `... 第0` だが、最後の空白で切り戻されて `...攻略` になる
        let names = [
            "[咲野日暮×ロウ] 中年魔術師の悠々自適なダンジョン攻略 第01巻",
            "[咲野日暮×ロウ] 中年魔術師の悠々自適なダンジョン攻略 第02巻",
            "[咲野日暮×ロウ] 中年魔術師の悠々自適なダンジョン攻略 第03巻",
        ];
        assert_eq!(
            longest_common_prefix(&names),
            "[咲野日暮×ロウ] 中年魔術師の悠々自適なダンジョン攻略"
        );
    }

    #[test]
    fn truncates_back_to_last_halfwidth_space() {
        // 共通部分が途中で切れている場合、最後の半角空白で切り戻す
        assert_eq!(longest_common_prefix(&["abc 01", "abc 02"]), "abc");
        assert_eq!(longest_common_prefix(&["abc   01", "abc   02"]), "abc");
    }

    #[test]
    fn truncates_back_to_last_fullwidth_space() {
        // 共通部分が途中で切れている場合、最後の全角空白で切り戻す
        assert_eq!(
            longest_common_prefix(&["タイトル\u{3000}第01巻", "タイトル\u{3000}第02巻"]),
            "タイトル"
        );
    }

    #[test]
    fn truncates_at_mixed_spaces() {
        // 半角と全角の混在 → 最後の空白で切り戻す
        assert_eq!(
            longest_common_prefix(&["abc \u{3000} 01", "abc \u{3000} 02"]),
            "abc"
        );
    }

    #[test]
    fn brackets_kept_truncation_at_space() {
        // `[A] Title-01`/`[A] Title-02` → 共通 `[A] Title-0`、最後の空白で切り戻し → `[A]`
        // 括弧は trim 対象ではないため残るが、空白での切り戻しが優先される
        assert_eq!(
            longest_common_prefix(&["[A] Title-01", "[A] Title-02"]),
            "[A]"
        );
    }

    #[test]
    fn no_whitespace_falls_back_to_raw_prefix() {
        // 空白を含まないケースでは、共通プレフィックスをそのまま fallback として返す
        // `[A]-01`/`[A]-02` → 共通 `[A]-0`、空白なし → `[A]-0`
        assert_eq!(
            longest_common_prefix(&["[A]-01", "[A]-02"]),
            "[A]-0"
        );
        // `Doraemon-Vol01`/`Doraemon-Vol02` → 共通 `Doraemon-Vol0`、空白なし → そのまま
        assert_eq!(
            longest_common_prefix(&["Doraemon-Vol01", "Doraemon-Vol02"]),
            "Doraemon-Vol0"
        );
    }

    #[test]
    fn japanese_without_whitespace_falls_back() {
        // 日本語で空白なし → 共通部分をそのまま fallback
        assert_eq!(
            longest_common_prefix(&["あいうえお1", "あいうえお2"]),
            "あいうえお"
        );
    }

    #[test]
    fn unicode_char_boundary_safety() {
        // サロゲートペア（絵文字）でも char 単位で安全に動作
        assert_eq!(longest_common_prefix(&["😀a", "😀b"]), "😀");
        // 絵文字 + 空白 + 差分の場合は、空白で切り戻す
        assert_eq!(longest_common_prefix(&["😀 a", "😀 b"]), "😀");
    }

    #[test]
    fn prefix_entirely_whitespace_returns_empty() {
        // 共通部分が全てスペース → 完全 prefix 扱いではないので空白で切り戻し → 結果は空
        assert_eq!(longest_common_prefix(&["   abc", "   xyz"]), "");
        assert_eq!(
            longest_common_prefix(&["\u{3000}\u{3000}abc", "\u{3000}\u{3000}xyz"]),
            ""
        );
    }

    #[test]
    fn one_string_is_complete_prefix_of_another() {
        // 一方が他方の完全 prefix → そのまま返す（短い方の末尾スペースだけ trim）
        assert_eq!(longest_common_prefix(&["abc", "abcd"]), "abc");
        assert_eq!(longest_common_prefix(&["abcd", "abc"]), "abc");
    }

    #[test]
    fn three_or_more_strings_no_whitespace() {
        // 3 件以上で最短の共通部分を採用、空白なしなので fallback
        assert_eq!(
            longest_common_prefix(&["prefix_x_01", "prefix_x_02", "prefix_y_01"]),
            "prefix_"
        );
    }

    // ── Phase 14.3: プラン生成ロジック ──────────────────────────────

    fn spec(prefix: &str, suffix: &str, digits: usize, start: u64, step: u64) -> GroupRenameSpec {
        GroupRenameSpec {
            prefix: prefix.to_string(),
            suffix: suffix.to_string(),
            digits,
            start,
            step,
            numbering: Numbering::Decimal,
        }
    }

    #[test]
    fn format_group_rename_decimal() {
        let s = spec("", "", 2, 1, 1);
        assert_eq!(format_group_rename(&s, 0), "01");
        assert_eq!(format_group_rename(&s, 1), "02");
        assert_eq!(format_group_rename(&s, 2), "03");
    }

    #[test]
    fn format_group_rename_with_prefix_suffix() {
        let s = spec("第", "巻", 2, 1, 1);
        assert_eq!(format_group_rename(&s, 0), "第01巻");
        assert_eq!(format_group_rename(&s, 1), "第02巻");
    }

    #[test]
    fn format_group_rename_step_2() {
        let s = spec("v", "", 1, 0, 2);
        // value = 0, 2, 4
        assert_eq!(format_group_rename(&s, 0), "v0");
        assert_eq!(format_group_rename(&s, 1), "v2");
        assert_eq!(format_group_rename(&s, 2), "v4");
    }

    #[test]
    fn format_group_rename_hex() {
        let s = GroupRenameSpec {
            prefix: "0x".into(),
            suffix: "".into(),
            digits: 2,
            start: 10,
            step: 1,
            numbering: Numbering::Hex,
        };
        assert_eq!(format_group_rename(&s, 0), "0x0A");
        assert_eq!(format_group_rename(&s, 5), "0x0F");
        assert_eq!(format_group_rename(&s, 6), "0x10");
    }

    #[test]
    fn build_plan_sorts_ascending_and_assigns_seq() {
        let names = vec!["第03巻".to_string(), "第01巻".to_string(), "第02巻".to_string()];
        let s = spec("", "", 2, 1, 1);
        let plan = build_plan("/work", &names, "group", Some(&s));

        assert_eq!(plan.items.len(), 3);
        assert_eq!(plan.items[0].original_name, "第01巻");
        assert_eq!(plan.items[0].renamed, "01");
        assert_eq!(plan.items[1].original_name, "第02巻");
        assert_eq!(plan.items[1].renamed, "02");
        assert_eq!(plan.items[2].original_name, "第03巻");
        assert_eq!(plan.items[2].renamed, "03");
    }

    #[test]
    fn build_plan_without_rename_keeps_original_names() {
        let names = vec!["b".to_string(), "a".to_string()];
        let plan = build_plan("/work", &names, "group", None);
        assert_eq!(plan.items[0].renamed, "a");
        assert_eq!(plan.items[1].renamed, "b");
    }

    #[test]
    fn build_plan_paths_use_proper_separators() {
        let names = vec!["a".to_string(), "b".to_string()];
        let plan = build_plan("/work", &names, "group", None);
        // パスセパレータは OS 依存だが、サブパスの構造は確認できる
        assert!(plan.items[0].original_path.contains("a"));
        assert!(plan.items[0].final_path.contains("group"));
        assert!(plan.items[0].final_path.ends_with("a"));
    }

    #[test]
    fn find_renamed_collision_detects_duplicate() {
        // step=0 などで全行が同じ連番値 → 重複
        let names = vec!["a".to_string(), "b".to_string()];
        let s = spec("v", "", 2, 1, 0); // step=0 → 全て "v01"
        let plan = build_plan("/work", &names, "g", Some(&s));
        assert_eq!(find_renamed_collision(&plan), Some("v01".to_string()));
    }

    #[test]
    fn find_renamed_collision_returns_none_for_unique() {
        let names = vec!["a".to_string(), "b".to_string()];
        let s = spec("v", "", 2, 1, 1);
        let plan = build_plan("/work", &names, "g", Some(&s));
        assert_eq!(find_renamed_collision(&plan), None);
    }
}
