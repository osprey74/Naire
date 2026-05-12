use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::filter::DisplayFilter;
use crate::rename::advanced::{apply_regex, wildcard_to_regex};
use crate::rename::builtin::{apply_builtin, BuiltinOp};
use crate::rename::char_convert;
use crate::rename::sequence::Numbering;
use crate::rename::undo::{undo_ops, OpView};
use crate::rename::variables::FileContext;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TargetType {
    File,
    Folder,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreviewItem {
    pub original: String,
    pub renamed: String,
    pub folder: String,
    pub path: String,
    pub is_changed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RenameOp {
    pub old_path: String,
    pub new_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RenameRecord {
    pub id: String,
    pub timestamp: String,
    pub ops: Vec<RenameOp>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StepItem {
    pub path: String,
    pub original_name: String,
    pub current_name: String,
    pub folder: String,
    pub size: u64,
    pub mtime: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SequenceConfigDto {
    pub start: u64,
    pub step: u64,
    pub reset_per_folder: bool,
    pub numbering: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RenameStepDto {
    pub kind: String,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub replace: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub op: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RenameItemDto {
    pub path: String,
    pub new_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Macro {
    pub id: String,
    pub name: String,
    pub steps: Vec<RenameStepDto>,
    pub created_at: String,
    pub updated_at: String,
}

/// コンパイル済みのリネームステップ。
enum CompiledStep {
    Regex { re: Regex, replace: String },
    Builtin { op: BuiltinOp },
    CharConvert { mapping: Vec<(String, String)> },
}

impl CompiledStep {
    fn apply(&self, current: &str, ctx: &FileContext) -> String {
        match self {
            CompiledStep::Regex { re, replace } => apply_regex(re, replace, current, ctx),
            CompiledStep::Builtin { op } => apply_builtin(op, current, ctx),
            CompiledStep::CharConvert { mapping } => char_convert::apply(current, mapping),
        }
    }
}

fn compile_steps(steps: &[RenameStepDto]) -> Result<Vec<CompiledStep>, String> {
    let mut out = Vec::new();
    for s in steps {
        match s.kind.as_str() {
            "regex" => {
                let search = s.search.as_deref().unwrap_or("");
                let replace = s.replace.clone().unwrap_or_default();
                if search.is_empty() {
                    continue;
                }
                let re = Regex::new(search).map_err(|e| format!("正規表現エラー: {}", e))?;
                out.push(CompiledStep::Regex { re, replace });
            }
            "wildcard" => {
                // ワイルドカード `?` `*` を正規表現に変換して、以降は regex と同じ機構で処理する。
                let search = s.search.as_deref().unwrap_or("");
                let replace = s.replace.clone().unwrap_or_default();
                if search.is_empty() {
                    continue;
                }
                let regex_pattern = wildcard_to_regex(search);
                let re = Regex::new(&regex_pattern)
                    .map_err(|e| format!("ワイルドカード変換エラー: {}", e))?;
                out.push(CompiledStep::Regex { re, replace });
            }
            "builtin" => {
                let op_value = s
                    .op
                    .clone()
                    .ok_or_else(|| "builtin ステップに op フィールドがありません".to_string())?;
                let op: BuiltinOp = serde_json::from_value(op_value)
                    .map_err(|e| format!("builtin op パース失敗: {}", e))?;
                out.push(CompiledStep::Builtin { op });
            }
            "char_convert" => {
                let from = s.from.as_deref().unwrap_or("");
                let to = s.to.as_deref().unwrap_or("");
                if from.is_empty() {
                    continue;
                }
                let mapping = char_convert::build_mapping(from, to);
                if mapping.is_empty() {
                    continue;
                }
                out.push(CompiledStep::CharConvert { mapping });
            }
            other => {
                return Err(format!("未実装のステップ種別: {}", other));
            }
        }
    }
    Ok(out)
}

fn enumerate_entries(
    folder: &str,
    target: &TargetType,
    recursive: bool,
    depth: u32,
    filter: &str,
) -> Result<Vec<PreviewItem>, String> {
    let display_filter = DisplayFilter::parse(filter)?;
    let root = PathBuf::from(folder);
    if !root.is_dir() {
        return Err(format!("フォルダが存在しません: {}", folder));
    }

    let base = WalkDir::new(&root).min_depth(1);
    let walker = if !recursive {
        base.max_depth(1)
    } else if depth == 0 {
        base
    } else {
        base.max_depth(depth as usize)
    };

    let mut items = Vec::new();
    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        let file_type = entry.file_type();
        let matches_target = match target {
            TargetType::File => file_type.is_file(),
            TargetType::Folder => file_type.is_dir(),
        };
        if !matches_target {
            continue;
        }

        let name = entry.file_name().to_string_lossy().into_owned();
        if !display_filter.matches(&name) {
            continue;
        }

        let parent = entry.path().parent().unwrap_or(root.as_path());
        let folder_rel = parent
            .strip_prefix(&root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();

        items.push(PreviewItem {
            original: name.clone(),
            renamed: name,
            folder: folder_rel,
            path: entry.path().to_string_lossy().into_owned(),
            is_changed: false,
        });
    }

    items.sort_by(|a, b| {
        (a.folder.as_str(), a.original.as_str()).cmp(&(b.folder.as_str(), b.original.as_str()))
    });
    Ok(items)
}

/// 列挙したエントリにステップを適用して `renamed` と `is_changed` を埋める。
/// `selected_indexes` が空・全件のときは全行に適用、部分選択時は選択行のみに
/// 適用する（非選択行は `renamed = original`、`is_changed = false`）。
///
/// 連番カウンタはステップ適用対象の行を昇順に走査して進める。非対象行はカウンタを
/// 消費しないため、選択行のみで連番が連続する。`reset_per_folder=true` なら
/// フォルダが切り替わるタイミングでカウンタを `start` に戻す。
fn build_preview(
    folder: &str,
    steps: &[RenameStepDto],
    target: &TargetType,
    recursive: bool,
    depth: u32,
    filter: &str,
    seq: &SequenceConfigDto,
    selected_indexes: &[usize],
) -> Result<Vec<PreviewItem>, String> {
    let mut items = enumerate_entries(folder, target, recursive, depth, filter)?;

    if steps.is_empty() {
        return Ok(items);
    }

    let compiled = compile_steps(steps)?;
    if compiled.is_empty() {
        return Ok(items);
    }

    let apply_to_all = selected_indexes.is_empty() || selected_indexes.len() == items.len();
    let selected: HashSet<usize> = if apply_to_all {
        HashSet::new()
    } else {
        selected_indexes.iter().copied().collect()
    };

    let numbering = Numbering::parse(&seq.numbering);
    let step_size = seq.step.max(1);
    let mut counter = seq.start;
    let mut applied_index: u64 = 0;
    let mut last_folder: Option<String> = None;

    for (idx, item) in items.iter_mut().enumerate() {
        if !apply_to_all && !selected.contains(&idx) {
            continue;
        }

        // フォルダごとリセット: 直前に適用した行と異なるフォルダなら
        // global counter / applied_index を初期化する。
        if seq.reset_per_folder {
            match &last_folder {
                Some(prev) if prev == &item.folder => {}
                _ => {
                    counter = seq.start;
                    applied_index = 0;
                }
            }
        }

        let path = PathBuf::from(&item.path);
        let mut ctx = FileContext::from_path(&path);
        ctx.seq_value = counter;
        ctx.seq_numbering = numbering;
        ctx.applied_index = applied_index;

        // ファイル metadata から size / mtime を取得（取れなければデフォルト値）
        if let Ok(meta) = std::fs::metadata(&path) {
            ctx.size = meta.len();
            if let Ok(modified) = meta.modified() {
                ctx.mtime = chrono::DateTime::<chrono::Local>::from(modified);
            }
        }

        let mut current = item.original.clone();
        for step in &compiled {
            current = step.apply(&current, &ctx);
        }
        item.renamed = current;
        item.is_changed = item.renamed != item.original;

        last_folder = Some(item.folder.clone());
        counter = counter.saturating_add(step_size);
        applied_index = applied_index.saturating_add(1);
    }

    Ok(items)
}

#[tauri::command]
pub async fn list_entries(
    folder: String,
    target: TargetType,
    recursive: bool,
    depth: u32,
    filter: String,
) -> Result<Vec<String>, String> {
    let items = enumerate_entries(&folder, &target, recursive, depth, &filter)?;
    Ok(items.into_iter().map(|p| p.original).collect())
}

#[tauri::command]
pub async fn preview_rename(
    folder: String,
    steps: Vec<RenameStepDto>,
    target: TargetType,
    recursive: bool,
    depth: u32,
    filter: String,
    seq: SequenceConfigDto,
    selected_indexes: Vec<usize>,
) -> Result<Vec<PreviewItem>, String> {
    build_preview(
        &folder,
        &steps,
        &target,
        recursive,
        depth,
        &filter,
        &seq,
        &selected_indexes,
    )
}

#[tauri::command]
pub async fn execute_rename(
    folder: String,
    steps: Vec<RenameStepDto>,
    target: TargetType,
    recursive: bool,
    depth: u32,
    filter: String,
    seq: SequenceConfigDto,
    selected_indexes: Vec<usize>,
) -> Result<RenameRecord, String> {
    let items = build_preview(
        &folder,
        &steps,
        &target,
        recursive,
        depth,
        &filter,
        &seq,
        &selected_indexes,
    )?;

    let changes: Vec<&PreviewItem> = items.iter().filter(|i| i.is_changed).collect();
    if changes.is_empty() {
        return Err("変更対象がありません".into());
    }

    // 1) バッチ内でターゲットパス重複がないか検証
    let mut targets: HashMap<String, String> = HashMap::new();
    let mut planned: Vec<(String, String)> = Vec::with_capacity(changes.len());
    for item in &changes {
        let old_path = PathBuf::from(&item.path);
        let new_path = old_path.with_file_name(&item.renamed);
        let new_path_str = new_path.to_string_lossy().into_owned();
        if let Some(existing) = targets.insert(new_path_str.clone(), item.path.clone()) {
            return Err(format!(
                "コンフリクト: {} と {} が同じ名前 {} にリネームされます",
                existing, item.path, new_path_str
            ));
        }
        planned.push((item.path.clone(), new_path_str));
    }

    // 2) チェーン検出（new path が同じバッチ内の別 item の old path と一致）
    let old_paths: HashSet<&String> = planned.iter().map(|(o, _)| o).collect();
    for (old_path, new_path) in &planned {
        if old_path != new_path && old_paths.contains(new_path) {
            return Err(format!(
                "リネーム順序の循環があります: {} → {}（バッチ分割が必要）",
                old_path, new_path
            ));
        }
    }

    // 3) バッチ外の既存ファイルとの衝突を検証
    for (old_path, new_path) in &planned {
        if old_path == new_path {
            continue;
        }
        if Path::new(new_path).exists() {
            return Err(format!("既に存在するファイル: {}", new_path));
        }
    }

    // 4) 実行
    let mut ops = Vec::with_capacity(planned.len());
    for (old_path, new_path) in planned {
        std::fs::rename(&old_path, &new_path).map_err(|e| {
            format!(
                "リネーム失敗 {} → {}: {} （これまでに {} 件成功）",
                old_path,
                new_path,
                e,
                ops.len()
            )
        })?;
        ops.push(RenameOp {
            old_path,
            new_path,
        });
    }

    Ok(RenameRecord {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Local::now().to_rfc3339(),
        ops,
    })
}

#[tauri::command]
pub async fn init_macro_items(
    _folder: String,
    _target: TargetType,
    _recursive: bool,
    _depth: u32,
    _filter: String,
) -> Result<Vec<StepItem>, String> {
    Err("not implemented".into())
}

#[tauri::command]
pub async fn apply_macro_step(
    _items: Vec<StepItem>,
    _step: RenameStepDto,
    _seq: SequenceConfigDto,
    _selected_indexes: Vec<usize>,
) -> Result<Vec<String>, String> {
    Err("not implemented".into())
}

#[tauri::command]
pub async fn apply_rename_to_filesystem(
    _items: Vec<RenameItemDto>,
) -> Result<RenameRecord, String> {
    Err("not implemented".into())
}

/// `RenameRecord` を逆適用する。実装は `rename/undo.rs::undo_ops` を参照。
#[tauri::command]
pub async fn undo_rename(record: RenameRecord) -> Result<(), String> {
    let views: Vec<OpView<'_>> = record
        .ops
        .iter()
        .map(|op| OpView {
            old_path: &op.old_path,
            new_path: &op.new_path,
        })
        .collect();
    undo_ops(&views)
}

#[tauri::command]
pub async fn export_macros(
    _app: tauri::AppHandle,
    _macros: Vec<Macro>,
) -> Result<(), String> {
    Err("not implemented".into())
}

#[tauri::command]
pub async fn import_macros(_app: tauri::AppHandle) -> Result<Vec<Macro>, String> {
    Err("not implemented".into())
}
