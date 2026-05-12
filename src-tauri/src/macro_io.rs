//! マクロ JSON のインポート／エクスポート。
//!
//! HANDOFF 仕様:
//! - 単一マクロは `Macro` オブジェクト、複数は `Macro[]` 配列
//! - インポート時は `id` を必ず再生成（重複防止）
//! - エクスポートファイル名: 単一 = `naire_macro_{name}.json`、複数 = `naire_macros.json`

use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use crate::commands::Macro;

/// インポート時に呼ばれる解析処理。テスト可能なよう dialog から切り出している。
///
/// 単一オブジェクト・配列のどちらも受け入れ、`id` を必ず再生成して返す。
pub fn parse_macros_json(text: &str) -> Result<Vec<Macro>, String> {
    let trimmed = text.trim_start();
    let macros: Vec<Macro> = if trimmed.starts_with('[') {
        serde_json::from_str(text).map_err(|e| format!("JSON パース失敗（配列）: {}", e))?
    } else {
        let m: Macro = serde_json::from_str(text)
            .map_err(|e| format!("JSON パース失敗（単体）: {}", e))?;
        vec![m]
    };
    Ok(macros
        .into_iter()
        .map(|mut m| {
            m.id = uuid::Uuid::new_v4().to_string();
            m
        })
        .collect())
}

/// macros を JSON 文字列にシリアライズする。
/// 単一マクロ（1 件）はオブジェクト、複数件は配列で出力する（HANDOFF 仕様）。
pub fn serialize_macros(macros: &[Macro]) -> Result<String, String> {
    let json = if macros.len() == 1 {
        serde_json::to_string_pretty(&macros[0])
    } else {
        serde_json::to_string_pretty(macros)
    };
    json.map_err(|e| format!("JSON 生成失敗: {}", e))
}

/// マクロ名から、ファイル名として使える文字列を生成する。
/// Windows / macOS の禁止文字を `_` に置換する。
pub fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            _ => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches('.');
    if trimmed.is_empty() {
        "macro".into()
    } else {
        trimmed.into()
    }
}

/// `FilePath` → ローカルパス文字列。
fn file_path_to_string(fp: tauri_plugin_dialog::FilePath) -> Result<String, String> {
    // `tauri_plugin_dialog::FilePath` は Path / Url の enum だが Display 経由で
    // 文字列化できる。ローカルファイルダイアログ経由なので Path variant のはず。
    Ok(fp.to_string())
}

pub fn import_via_dialog(app: &AppHandle) -> Result<Vec<Macro>, String> {
    let fp = app
        .dialog()
        .file()
        .add_filter("Naire Macro", &["json"])
        .blocking_pick_file()
        .ok_or_else(|| "キャンセルされました".to_string())?;
    let path = file_path_to_string(fp)?;
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("読み込み失敗: {}: {}", path, e))?;
    parse_macros_json(&text)
}

pub fn export_via_dialog(app: &AppHandle, macros: Vec<Macro>) -> Result<(), String> {
    if macros.is_empty() {
        return Err("エクスポートするマクロがありません".into());
    }
    let default_name = if macros.len() == 1 {
        format!("naire_macro_{}.json", sanitize_filename(&macros[0].name))
    } else {
        "naire_macros.json".to_string()
    };
    let fp = app
        .dialog()
        .file()
        .add_filter("Naire Macro", &["json"])
        .set_file_name(&default_name)
        .blocking_save_file()
        .ok_or_else(|| "キャンセルされました".to_string())?;
    let path = file_path_to_string(fp)?;
    let json = serialize_macros(&macros)?;
    std::fs::write(&path, json).map_err(|e| format!("書き込み失敗: {}: {}", path, e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn macro_json_single() -> &'static str {
        r#"{
  "id": "old-id",
  "name": "test",
  "created_at": "2026-05-12T10:00:00Z",
  "updated_at": "2026-05-12T10:00:00Z",
  "steps": [
    {"kind": "regex", "search": "foo", "replace": "bar"}
  ]
}"#
    }

    fn macro_json_array() -> &'static str {
        r#"[
  {
    "id": "id1",
    "name": "first",
    "created_at": "2026-05-12T10:00:00Z",
    "updated_at": "2026-05-12T10:00:00Z",
    "steps": []
  },
  {
    "id": "id2",
    "name": "second",
    "created_at": "2026-05-12T10:00:00Z",
    "updated_at": "2026-05-12T10:00:00Z",
    "steps": []
  }
]"#
    }

    #[test]
    fn parse_single_object() {
        let macros = parse_macros_json(macro_json_single()).unwrap();
        assert_eq!(macros.len(), 1);
        assert_eq!(macros[0].name, "test");
        // id 再生成: 元の "old-id" ではない
        assert_ne!(macros[0].id, "old-id");
        assert_eq!(macros[0].steps.len(), 1);
        assert_eq!(macros[0].steps[0].kind, "regex");
    }

    #[test]
    fn parse_array() {
        let macros = parse_macros_json(macro_json_array()).unwrap();
        assert_eq!(macros.len(), 2);
        assert_eq!(macros[0].name, "first");
        assert_eq!(macros[1].name, "second");
        assert_ne!(macros[0].id, "id1");
        assert_ne!(macros[1].id, "id2");
        // 再生成された id 同士も別物
        assert_ne!(macros[0].id, macros[1].id);
    }

    #[test]
    fn parse_array_with_leading_whitespace() {
        let text = "\n\n  [{\"id\":\"x\",\"name\":\"y\",\"created_at\":\"\",\"updated_at\":\"\",\"steps\":[]}]";
        let macros = parse_macros_json(text).unwrap();
        assert_eq!(macros.len(), 1);
        assert_eq!(macros[0].name, "y");
    }

    #[test]
    fn parse_malformed_returns_error() {
        let err = parse_macros_json("not json").unwrap_err();
        assert!(err.contains("JSON パース失敗"));
    }

    #[test]
    fn serialize_single_as_object() {
        let m = Macro {
            id: "id1".into(),
            name: "test".into(),
            created_at: "2026-05-12T10:00:00Z".into(),
            updated_at: "2026-05-12T10:00:00Z".into(),
            steps: vec![],
        };
        let json = serialize_macros(&[m]).unwrap();
        assert!(json.trim_start().starts_with('{'));
        assert!(json.contains("\"name\": \"test\""));
    }

    #[test]
    fn serialize_multiple_as_array() {
        let make = |name: &str| Macro {
            id: format!("id-{}", name),
            name: name.into(),
            created_at: "".into(),
            updated_at: "".into(),
            steps: vec![],
        };
        let json = serialize_macros(&[make("a"), make("b")]).unwrap();
        assert!(json.trim_start().starts_with('['));
    }

    #[test]
    fn sanitize_filename_strips_path_chars() {
        assert_eq!(sanitize_filename("hello/world"), "hello_world");
        assert_eq!(sanitize_filename("a:b*c?d"), "a_b_c_d");
        assert_eq!(sanitize_filename("作者ソートキー"), "作者ソートキー");
    }

    #[test]
    fn sanitize_filename_falls_back_for_empty() {
        assert_eq!(sanitize_filename(""), "macro");
        assert_eq!(sanitize_filename("   "), "macro");
        assert_eq!(sanitize_filename("..."), "macro");
    }
}
