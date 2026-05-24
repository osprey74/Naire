use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::filter::DisplayFilter;
use crate::rename::advanced::{apply_regex, wildcard_to_regex};
use crate::rename::builtin::{apply_builtin, BuiltinOp};
use crate::rename::char_convert;
use crate::rename::group::{
    build_plan, find_renamed_collision, longest_common_prefix, GroupRenameSpec,
};
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

/// UNDO スタック上の 1 操作を表す。Phase 14 から enum 化。
/// - `Rename`: 通常のリネーム / 移動（既存コマンドが生成）
/// - `CreateDir`: フォルダ作成（`execute_group` のみが生成）
///
/// JSON 形式（serde `tag = "type"`、`rename_all = "snake_case"`）:
/// - `{"type": "rename", "old_path": "...", "new_path": "..."}`
/// - `{"type": "create_dir", "path": "..."}`
///
/// UNDO スタックは永続化しないため、シリアライズ形式の後方互換性は不要。
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RenameOp {
    Rename { old_path: String, new_path: String },
    CreateDir { path: String },
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
        ops.push(RenameOp::Rename {
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

/// マクロ実行の初期化: フォルダ列挙して `StepItem` 群を返す。
/// `original_name == current_name`、size / mtime は metadata から取得。
#[tauri::command]
pub async fn init_macro_items(
    folder: String,
    target: TargetType,
    recursive: bool,
    depth: u32,
    filter: String,
) -> Result<Vec<StepItem>, String> {
    let entries = enumerate_entries(&folder, &target, recursive, depth, &filter)?;
    let items: Vec<StepItem> = entries
        .into_iter()
        .map(|e| {
            let path = PathBuf::from(&e.path);
            let (size, mtime) = std::fs::metadata(&path)
                .map(|meta| {
                    let size = meta.len();
                    let mtime = meta
                        .modified()
                        .map(|t| chrono::DateTime::<chrono::Local>::from(t).to_rfc3339())
                        .unwrap_or_default();
                    (size, mtime)
                })
                .unwrap_or((0, String::new()));
            StepItem {
                path: e.path,
                original_name: e.original.clone(),
                current_name: e.original,
                folder: e.folder,
                size,
                mtime,
            }
        })
        .collect();
    Ok(items)
}

/// 1 ステップを items 全体に適用して、各 item の新しい `current_name` を返す。
/// 選択行のみに適用し、非選択行は元の `current_name` を維持する。
/// 連番カウンタとフォルダリセットの挙動は `build_preview` と同じ。
///
/// **マクロコンテキスト**: `FileContext.original_full` は `item.original_name` で
/// 上書きし、`\orig` 変数を step 0 名にバインドする。`\0 \t \e` は `current` 引数
/// （= 直前ステップ適用後の名前）から導出される。
#[tauri::command]
pub async fn apply_macro_step(
    items: Vec<StepItem>,
    step: RenameStepDto,
    seq: SequenceConfigDto,
    selected_indexes: Vec<usize>,
) -> Result<Vec<String>, String> {
    let compiled = compile_steps(std::slice::from_ref(&step))?;
    if compiled.is_empty() {
        return Ok(items.into_iter().map(|i| i.current_name).collect());
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

    let mut new_names = Vec::with_capacity(items.len());
    for (idx, item) in items.iter().enumerate() {
        if !apply_to_all && !selected.contains(&idx) {
            new_names.push(item.current_name.clone());
            continue;
        }

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
        // マクロセマンティクス: \orig は step 0 名を指す
        ctx.original_full = item.original_name.clone();
        ctx.seq_value = counter;
        ctx.seq_numbering = numbering;
        ctx.applied_index = applied_index;

        if let Ok(meta) = std::fs::metadata(&path) {
            ctx.size = meta.len();
            if let Ok(modified) = meta.modified() {
                ctx.mtime = chrono::DateTime::<chrono::Local>::from(modified);
            }
        }

        let mut current = item.current_name.clone();
        for s in &compiled {
            current = s.apply(&current, &ctx);
        }
        new_names.push(current);

        last_folder = Some(item.folder.clone());
        counter = counter.saturating_add(step_size);
        applied_index = applied_index.saturating_add(1);
    }

    Ok(new_names)
}

/// 確定済みの (path, new_name) 配列をディスクに反映する。
/// execute_rename と同様にバッチ内重複・チェーン・既存衝突を検証してから
/// `std::fs::rename` を順次実行し、`RenameRecord` を返す。
#[tauri::command]
pub async fn apply_rename_to_filesystem(
    items: Vec<RenameItemDto>,
) -> Result<RenameRecord, String> {
    // 変更のあるエントリのみ抽出
    let mut planned: Vec<(String, String)> = Vec::new();
    let mut targets: HashMap<String, String> = HashMap::new();
    for item in &items {
        let old_path = PathBuf::from(&item.path);
        let current_name = old_path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        if item.new_name == current_name {
            continue; // 変更なし
        }
        let new_path = old_path.with_file_name(&item.new_name);
        let new_path_str = new_path.to_string_lossy().into_owned();
        if let Some(existing) = targets.insert(new_path_str.clone(), item.path.clone()) {
            return Err(format!(
                "コンフリクト: {} と {} が同じ名前 {} にリネームされます",
                existing, item.path, new_path_str
            ));
        }
        planned.push((item.path.clone(), new_path_str));
    }

    if planned.is_empty() {
        return Err("変更対象がありません".into());
    }

    let old_paths: HashSet<&String> = planned.iter().map(|(o, _)| o).collect();
    for (old_path, new_path) in &planned {
        if old_path != new_path && old_paths.contains(new_path) {
            return Err(format!(
                "リネーム順序の循環があります: {} → {}（バッチ分割が必要）",
                old_path, new_path
            ));
        }
    }
    for (old_path, new_path) in &planned {
        if old_path == new_path {
            continue;
        }
        if Path::new(new_path).exists() {
            return Err(format!("既に存在するファイル: {}", new_path));
        }
    }

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
        ops.push(RenameOp::Rename {
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

/// `RenameRecord` を逆適用する。実装は `rename/undo.rs::undo_ops` を参照。
#[tauri::command]
pub async fn undo_rename(record: RenameRecord) -> Result<(), String> {
    let views: Vec<OpView<'_>> = record
        .ops
        .iter()
        .map(|op| match op {
            RenameOp::Rename { old_path, new_path } => OpView::Rename {
                old_path,
                new_path,
            },
            RenameOp::CreateDir { path } => OpView::CreateDir { path },
        })
        .collect();
    undo_ops(&views)
}

// ── フォルダ集約（Phase 14） ─────────────────────────────────────

/// 集約後の連番リネーム指定。定型 #1 `add_seq_str` と同一動作。
/// `numbering` はグローバル SequenceConfig から継承する想定（UI が現在値を送る）。
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GroupRenameDto {
    pub prefix: String,
    pub suffix: String,
    pub digits: u32,
    pub start: u64,
    pub step: u64,
    pub numbering: String,
}

/// 集約プレビューに含める 1 件分の情報。
#[derive(Serialize, Clone, Debug)]
pub struct GroupItemPreview {
    pub original_name: String,
    pub renamed: String,
    pub final_path: String,
}

/// 集約プレビュー結果。エラー時は `error: Some` で `items` は空。
/// `conflict` は集約フォルダ名が既存と衝突した場合に true（プレビュー自体は成立）。
#[derive(Serialize, Clone, Debug)]
pub struct GroupPreview {
    pub parent_folder: String,
    pub common_prefix: String,
    pub group_name: String,
    pub conflict: bool,
    pub items: Vec<GroupItemPreview>,
    pub error: Option<String>,
}

fn make_error_preview(
    parent_folder: String,
    common_prefix: String,
    group_name: String,
    msg: impl Into<String>,
) -> GroupPreview {
    GroupPreview {
        parent_folder,
        common_prefix,
        group_name,
        conflict: false,
        items: vec![],
        error: Some(msg.into()),
    }
}

fn dto_to_spec(dto: &GroupRenameDto) -> Result<GroupRenameSpec, String> {
    if dto.digits == 0 {
        return Err("連番桁数は 1 以上で指定してください".into());
    }
    if dto.step == 0 {
        return Err("連番ステップは 1 以上で指定してください".into());
    }
    Ok(GroupRenameSpec {
        prefix: dto.prefix.clone(),
        suffix: dto.suffix.clone(),
        digits: dto.digits as usize,
        start: dto.start,
        step: dto.step,
        numbering: Numbering::parse(&dto.numbering),
    })
}

/// 選択フォルダから共通プレフィックスとプレビュー情報を返す（FS は変更しない）。
///
/// `group_name` が None の場合は共通プレフィックスを既定値として使用。
/// バリデーション失敗時も Ok を返し、`error` フィールドにメッセージを格納する。
#[tauri::command]
pub async fn compute_group_preview(
    parent_folder: String,
    selected_names: Vec<String>,
    group_name: Option<String>,
    rename: Option<GroupRenameDto>,
) -> Result<GroupPreview, String> {
    compute_group_preview_impl(parent_folder, selected_names, group_name, rename)
}

/// `compute_group_preview` の同期実装（テスト用に分離）。
pub(crate) fn compute_group_preview_impl(
    parent_folder: String,
    selected_names: Vec<String>,
    group_name: Option<String>,
    rename: Option<GroupRenameDto>,
) -> Result<GroupPreview, String> {
    // 共通プレフィックス算出（選択 0/1 件でも値は出せる）
    let refs: Vec<&str> = selected_names.iter().map(|s| s.as_str()).collect();
    let common_prefix = longest_common_prefix(&refs);

    // 解決される group_name (UI 入力優先、None なら共通プレフィックス)
    let resolved_group_name = group_name
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| common_prefix.clone());

    // 選択件数バリデーション
    if selected_names.is_empty() {
        return Ok(make_error_preview(
            parent_folder,
            common_prefix,
            resolved_group_name,
            "選択フォルダがありません",
        ));
    }
    if selected_names.len() < 2 {
        return Ok(make_error_preview(
            parent_folder,
            common_prefix,
            resolved_group_name,
            "集約には 2 件以上のフォルダ選択が必要です",
        ));
    }

    // 全選択肢が parent_folder 直下に実在するフォルダか
    let parent_path = Path::new(&parent_folder);
    if !parent_path.is_dir() {
        return Ok(make_error_preview(
            parent_folder.clone(),
            common_prefix,
            resolved_group_name,
            format!("親フォルダが見つかりません: {}", parent_folder),
        ));
    }
    for name in &selected_names {
        let full = parent_path.join(name);
        if !full.exists() {
            return Ok(make_error_preview(
                parent_folder.clone(),
                common_prefix,
                resolved_group_name,
                format!("フォルダが見つかりません: {}", name),
            ));
        }
        if !full.is_dir() {
            return Ok(make_error_preview(
                parent_folder.clone(),
                common_prefix,
                resolved_group_name,
                format!("フォルダではありません: {}", name),
            ));
        }
    }

    // group_name の妥当性
    if resolved_group_name.is_empty() {
        return Ok(make_error_preview(
            parent_folder,
            common_prefix,
            resolved_group_name,
            "集約フォルダ名が空です",
        ));
    }
    if selected_names.iter().any(|n| n == &resolved_group_name) {
        return Ok(make_error_preview(
            parent_folder,
            common_prefix,
            resolved_group_name,
            "集約フォルダ名が選択中のフォルダ名と同一です",
        ));
    }

    // rename DTO → Spec 変換
    let rename_spec: Option<GroupRenameSpec> = match rename {
        Some(r) => match dto_to_spec(&r) {
            Ok(spec) => Some(spec),
            Err(msg) => {
                return Ok(make_error_preview(
                    parent_folder,
                    common_prefix,
                    resolved_group_name,
                    msg,
                ));
            }
        },
        None => None,
    };

    // 計画を構築
    let plan = build_plan(
        &parent_folder,
        &selected_names,
        &resolved_group_name,
        rename_spec.as_ref(),
    );

    // 連番リネーム後の重複チェック
    if let Some(dup) = find_renamed_collision(&plan) {
        return Ok(GroupPreview {
            parent_folder,
            common_prefix,
            group_name: resolved_group_name,
            conflict: false,
            items: plan
                .items
                .iter()
                .map(|i| GroupItemPreview {
                    original_name: i.original_name.clone(),
                    renamed: i.renamed.clone(),
                    final_path: i.final_path.clone(),
                })
                .collect(),
            error: Some(format!("連番リネーム結果が重複します: {}", dup)),
        });
    }

    // 衝突判定（集約フォルダが既存）
    let group_full = parent_path.join(&resolved_group_name);
    let conflict = group_full.exists();

    Ok(GroupPreview {
        parent_folder,
        common_prefix,
        group_name: resolved_group_name,
        conflict,
        items: plan
            .items
            .into_iter()
            .map(|i| GroupItemPreview {
                original_name: i.original_name,
                renamed: i.renamed,
                final_path: i.final_path,
            })
            .collect(),
        error: None,
    })
}

/// 集約フォルダ作成 + 選択フォルダ移動 + 内部の連番リネームを実行する。
///
/// 内部的に `compute_group_preview` を再利用してバリデーションを通したあと、
/// 計画通りに `mkdir` + `std::fs::rename` を実行する。move と rename は同一の
/// `std::fs::rename` 呼び出しに統合（最終パスへ直接移動）。
///
/// 戻り値の `RenameRecord` には `CreateDir + Rename×N` が記録され、UNDO で
/// 完全に巻き戻せる。
#[tauri::command]
pub async fn execute_group(
    parent_folder: String,
    selected_names: Vec<String>,
    group_name: String,
    rename: Option<GroupRenameDto>,
) -> Result<RenameRecord, String> {
    execute_group_impl(parent_folder, selected_names, group_name, rename)
}

/// `execute_group` の同期実装（テスト用に分離）。
pub(crate) fn execute_group_impl(
    parent_folder: String,
    selected_names: Vec<String>,
    group_name: String,
    rename: Option<GroupRenameDto>,
) -> Result<RenameRecord, String> {
    // バリデーション（プレビュー経由）
    let preview = compute_group_preview_impl(
        parent_folder.clone(),
        selected_names.clone(),
        Some(group_name.clone()),
        rename.clone(),
    )?;

    if let Some(err) = preview.error {
        return Err(err);
    }
    if preview.conflict {
        return Err(format!(
            "集約フォルダ名が既存と衝突します: {}",
            preview.group_name
        ));
    }
    if preview.items.is_empty() {
        return Err("集約対象がありません".into());
    }

    // 計画を再構築（preview.items は GroupItemPreview なので、ファイル I/O 用に
    // 元の build_plan を呼び直す方が型変換コストなしで扱いやすい）
    let rename_spec = match rename {
        Some(ref r) => Some(dto_to_spec(r)?),
        None => None,
    };
    let plan = build_plan(
        &parent_folder,
        &selected_names,
        &preview.group_name,
        rename_spec.as_ref(),
    );

    // 集約フォルダ作成
    let group_path = Path::new(&plan.group_path).to_path_buf();
    std::fs::create_dir(&group_path).map_err(|e| {
        format!(
            "集約フォルダ作成失敗 {}: {}",
            group_path.display(),
            e
        )
    })?;

    let mut ops: Vec<RenameOp> = Vec::with_capacity(1 + plan.items.len());
    ops.push(RenameOp::CreateDir {
        path: plan.group_path.clone(),
    });

    // 移動 + 連番リネーム（単一の std::fs::rename で実行）
    for item in &plan.items {
        std::fs::rename(&item.original_path, &item.final_path).map_err(|e| {
            format!(
                "移動失敗 {} → {}: {} （これまでに {} 件処理済み、CreateDir 含む）",
                item.original_path,
                item.final_path,
                e,
                ops.len()
            )
        })?;
        ops.push(RenameOp::Rename {
            old_path: item.original_path.clone(),
            new_path: item.final_path.clone(),
        });
    }

    Ok(RenameRecord {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Local::now().to_rfc3339(),
        ops,
    })
}

#[tauri::command]
pub async fn export_macros(
    app: tauri::AppHandle,
    macros: Vec<Macro>,
) -> Result<(), String> {
    crate::macro_io::export_via_dialog(&app, macros)
}

#[tauri::command]
pub async fn import_macros(app: tauri::AppHandle) -> Result<Vec<Macro>, String> {
    crate::macro_io::import_via_dialog(&app)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DirNode {
    pub name: String,
    pub path: String,
    pub has_children: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FolderStats {
    pub file_count: u64,
    pub total_size: u64,
}

/// 指定フォルダ配下の総ファイル数と総バイト数を再帰的に集計する。
/// 権限エラー等で読めなかったエントリはスキップ（呼び出し側に伝播させない）。
#[tauri::command]
pub async fn get_folder_stats(path: String) -> Result<FolderStats, String> {
    let p = PathBuf::from(&path);
    if !p.is_dir() {
        return Err(format!("フォルダが存在しません: {}", path));
    }
    let mut file_count: u64 = 0;
    let mut total_size: u64 = 0;
    for entry in WalkDir::new(&p)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            file_count = file_count.saturating_add(1);
            if let Ok(meta) = entry.metadata() {
                total_size = total_size.saturating_add(meta.len());
            }
        }
    }
    Ok(FolderStats {
        file_count,
        total_size,
    })
}

/// 指定フォルダを OS 標準のゴミ箱（Windows: Recycle Bin / macOS: Trash）へ移動する。
/// 物理削除ではないため、ユーザは OS のゴミ箱から復元可能。アプリ内 UNDO とは別系統。
#[tauri::command]
pub async fn move_folder_to_trash(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("フォルダが見つかりません: {}", path));
    }
    if !p.is_dir() {
        return Err(format!("フォルダではありません: {}", path));
    }
    trash::delete(&p).map_err(|e| format!("ゴミ箱への移動に失敗しました: {}", e))?;
    Ok(())
}

/// 単一フォルダのリネーム。`old_path` の親はそのままで、basename を `new_name` に変更する。
/// 戻り値は UNDO 可能な `RenameRecord`。
#[tauri::command]
pub async fn rename_folder(
    old_path: String,
    new_name: String,
) -> Result<RenameRecord, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err("フォルダ名が空です".into());
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err("フォルダ名にパス区切り文字は使えません".into());
    }

    let old = PathBuf::from(&old_path);
    if !old.is_dir() {
        return Err(format!("フォルダが見つかりません: {}", old_path));
    }
    let current_name = old
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if current_name == trimmed {
        return Err("名前が変わっていません".into());
    }
    let new_path = old.with_file_name(trimmed);
    let new_path_str = new_path.to_string_lossy().into_owned();
    if new_path.exists() {
        return Err(format!("既に存在します: {}", new_path_str));
    }

    std::fs::rename(&old, &new_path).map_err(|e| {
        format!("リネーム失敗 {} → {}: {}", old_path, new_path_str, e)
    })?;

    Ok(RenameRecord {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Local::now().to_rfc3339(),
        ops: vec![RenameOp::Rename {
            old_path,
            new_path: new_path_str,
        }],
    })
}

/// 即子フォルダのうち最初の 1 件を見つけたら true。権限エラーは false 扱い。
fn has_subdir(path: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(path) else {
        return false;
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if !ft.is_dir() {
            continue;
        }
        #[cfg(unix)]
        {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with('.') {
                continue;
            }
        }
        return true;
    }
    false
}

#[cfg(windows)]
fn tree_roots() -> Vec<DirNode> {
    let mut roots = Vec::new();
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        if Path::new(&drive).is_dir() {
            roots.push(DirNode {
                name: drive.clone(),
                path: drive.clone(),
                has_children: has_subdir(Path::new(&drive)),
            });
        }
    }
    roots
}

#[cfg(unix)]
fn tree_roots() -> Vec<DirNode> {
    let mut roots = Vec::new();
    if let Some(home_os) = std::env::var_os("HOME") {
        let home = PathBuf::from(&home_os);
        if home.is_dir() {
            let name = home
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Home".to_string());
            roots.push(DirNode {
                name,
                path: home.to_string_lossy().into_owned(),
                has_children: has_subdir(&home),
            });
        }
    }
    #[cfg(target_os = "macos")]
    {
        // /Volumes 配下: 外部ディスク / SMB / AFP 等のネットワークマウントを含む。
        // 起動ボリュームは `/Volumes/<name>` から `/` への symlink として現れる場合があり
        // 重複しうるが、ユーザがシステムルートを参照したいケースもあるためそのまま含める。
        if let Ok(entries) = std::fs::read_dir("/Volumes") {
            let mut volumes: Vec<DirNode> = entries
                .flatten()
                .filter_map(|entry| {
                    let path = entry.path();
                    if !path.is_dir() {
                        return None;
                    }
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if name.starts_with('.') {
                        return None;
                    }
                    Some(DirNode {
                        name,
                        path: path.to_string_lossy().into_owned(),
                        has_children: has_subdir(&path),
                    })
                })
                .collect();
            volumes.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            roots.extend(volumes);
        }
    }
    roots
}

/// `path` 直下のサブフォルダを列挙する。`path` が None ならルート（Windows: ドライブ
/// 一覧 / macOS・Linux: $HOME 1 件）を返す。Unix 系では `.` 始まりの隠しフォルダを除外。
#[tauri::command]
pub fn list_folder_tree(path: Option<String>) -> Result<Vec<DirNode>, String> {
    let Some(path) = path else {
        return Ok(tree_roots());
    };
    let p = Path::new(&path);
    if !p.is_dir() {
        return Err(format!("フォルダが存在しません: {}", path));
    }
    let entries = std::fs::read_dir(p).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if !ft.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        #[cfg(unix)]
        if name.starts_with('.') {
            continue;
        }
        let child_path = entry.path();
        out.push(DirNode {
            name,
            path: child_path.to_string_lossy().into_owned(),
            has_children: has_subdir(&child_path),
        });
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

#[cfg(test)]
mod group_tests {
    //! Phase 14.3 — フォルダ集約 (compute_group_preview / execute_group) 統合テスト。
    //! Tauri コマンドの async ラッパーは pass-through なので、同期 `_impl` 関数を直接テストする。

    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn tempdir() -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "naire-group-test-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_dirs(parent: &PathBuf, names: &[&str]) {
        for n in names {
            fs::create_dir(parent.join(n)).unwrap();
        }
    }

    fn default_rename() -> GroupRenameDto {
        GroupRenameDto {
            prefix: String::new(),
            suffix: String::new(),
            digits: 2,
            start: 1,
            step: 1,
            numbering: "decimal".into(),
        }
    }

    // ── compute_group_preview ─────────────────────────────────────

    #[test]
    fn preview_normal_case() {
        let dir = tempdir();
        make_dirs(
            &dir,
            &[
                "[A] Title 第01巻",
                "[A] Title 第02巻",
                "[A] Title 第03巻",
            ],
        );
        let parent = dir.to_string_lossy().into_owned();
        let names: Vec<String> = vec![
            "[A] Title 第01巻".into(),
            "[A] Title 第02巻".into(),
            "[A] Title 第03巻".into(),
        ];

        let preview = compute_group_preview_impl(
            parent.clone(),
            names,
            None,
            Some(default_rename()),
        )
        .unwrap();

        assert_eq!(preview.error, None);
        assert!(!preview.conflict);
        assert_eq!(preview.common_prefix, "[A] Title");
        assert_eq!(preview.group_name, "[A] Title");
        assert_eq!(preview.items.len(), 3);
        assert_eq!(preview.items[0].renamed, "01");
        assert_eq!(preview.items[1].renamed, "02");
        assert_eq!(preview.items[2].renamed, "03");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn preview_with_prefix_suffix() {
        let dir = tempdir();
        make_dirs(&dir, &["a", "b"]);
        let parent = dir.to_string_lossy().into_owned();
        let rename = GroupRenameDto {
            prefix: "第".into(),
            suffix: "巻".into(),
            digits: 2,
            start: 1,
            step: 1,
            numbering: "decimal".into(),
        };

        let preview = compute_group_preview_impl(
            parent,
            vec!["a".into(), "b".into()],
            Some("group".into()),
            Some(rename),
        )
        .unwrap();

        assert_eq!(preview.error, None);
        assert_eq!(preview.items[0].renamed, "第01巻");
        assert_eq!(preview.items[1].renamed, "第02巻");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn preview_empty_selection_errors() {
        let preview =
            compute_group_preview_impl("/tmp".into(), vec![], None, None).unwrap();
        assert!(preview.error.as_deref().unwrap().contains("選択フォルダ"));
        assert!(preview.items.is_empty());
    }

    #[test]
    fn preview_single_selection_errors() {
        let preview = compute_group_preview_impl(
            "/tmp".into(),
            vec!["only".into()],
            None,
            None,
        )
        .unwrap();
        assert!(preview.error.as_deref().unwrap().contains("2 件以上"));
    }

    #[test]
    fn preview_missing_folder_errors() {
        let dir = tempdir();
        // 親フォルダは存在するが、子フォルダは作らない
        let parent = dir.to_string_lossy().into_owned();
        let preview = compute_group_preview_impl(
            parent,
            vec!["nope1".into(), "nope2".into()],
            None,
            None,
        )
        .unwrap();
        assert!(preview.error.as_deref().unwrap().contains("見つかりません"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn preview_file_not_dir_errors() {
        let dir = tempdir();
        fs::write(dir.join("a"), "content").unwrap();
        fs::create_dir(dir.join("b")).unwrap();
        let parent = dir.to_string_lossy().into_owned();
        let preview = compute_group_preview_impl(
            parent,
            vec!["a".into(), "b".into()],
            None,
            None,
        )
        .unwrap();
        assert!(preview.error.as_deref().unwrap().contains("フォルダではありません"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn preview_conflict_when_group_name_exists() {
        let dir = tempdir();
        make_dirs(&dir, &["a", "b", "existing"]);
        let parent = dir.to_string_lossy().into_owned();
        let preview = compute_group_preview_impl(
            parent,
            vec!["a".into(), "b".into()],
            Some("existing".into()),
            Some(default_rename()),
        )
        .unwrap();
        assert_eq!(preview.error, None);
        assert!(preview.conflict);
        // プレビューアイテムは返る（衝突情報も含めて UI が判断）
        assert_eq!(preview.items.len(), 2);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn preview_rename_duplicate_errors() {
        let dir = tempdir();
        make_dirs(&dir, &["a", "b"]);
        let parent = dir.to_string_lossy().into_owned();
        let mut r = default_rename();
        r.step = 1;
        r.start = 1;
        // step=0 は dto_to_spec で弾かれるので、ここでは強制的に重複を起こすために
        // prefix/suffix が同じになるパターンを作る…が、build_plan は idx で区別するので
        // 連番値が違えば重複しない。step=0 を渡せばエラーになる:
        r.step = 0;
        let preview = compute_group_preview_impl(
            parent,
            vec!["a".into(), "b".into()],
            Some("group".into()),
            Some(r),
        )
        .unwrap();
        assert!(preview.error.as_deref().unwrap().contains("ステップ"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn preview_empty_group_name_errors() {
        let dir = tempdir();
        make_dirs(&dir, &["a", "b"]);
        let parent = dir.to_string_lossy().into_owned();
        let preview = compute_group_preview_impl(
            parent,
            vec!["a".into(), "b".into()],
            Some("   ".into()),
            None,
        )
        .unwrap();
        assert!(preview.error.as_deref().unwrap().contains("集約フォルダ名"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn preview_no_rename_keeps_original_names() {
        let dir = tempdir();
        make_dirs(&dir, &["a", "b"]);
        let parent = dir.to_string_lossy().into_owned();
        let preview = compute_group_preview_impl(
            parent,
            vec!["a".into(), "b".into()],
            Some("group".into()),
            None,
        )
        .unwrap();
        assert_eq!(preview.error, None);
        assert_eq!(preview.items[0].renamed, "a");
        assert_eq!(preview.items[1].renamed, "b");
        fs::remove_dir_all(&dir).ok();
    }

    // ── execute_group ──────────────────────────────────────────────

    #[test]
    fn execute_normal_case() {
        let dir = tempdir();
        let names = ["[A] Title 第03巻", "[A] Title 第01巻", "[A] Title 第02巻"];
        make_dirs(&dir, &names);
        let parent = dir.to_string_lossy().into_owned();

        let record = execute_group_impl(
            parent.clone(),
            names.iter().map(|s| s.to_string()).collect(),
            "[A] Title".into(),
            Some(default_rename()),
        )
        .unwrap();

        // 集約フォルダが作成され、3 フォルダが連番で配置される
        let group_dir = dir.join("[A] Title");
        assert!(group_dir.is_dir());
        assert!(group_dir.join("01").is_dir());
        assert!(group_dir.join("02").is_dir());
        assert!(group_dir.join("03").is_dir());
        // 元のフォルダはもう存在しない
        for name in &names {
            assert!(!dir.join(name).exists());
        }

        // RenameRecord は CreateDir 1 件 + Rename 3 件 = 4 件
        assert_eq!(record.ops.len(), 4);
        match &record.ops[0] {
            RenameOp::CreateDir { path } => assert!(path.contains("[A] Title")),
            _ => panic!("最初の op は CreateDir のはず"),
        }

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn execute_blocked_by_conflict() {
        let dir = tempdir();
        make_dirs(&dir, &["a", "b", "existing"]);
        let parent = dir.to_string_lossy().into_owned();

        let result = execute_group_impl(
            parent,
            vec!["a".into(), "b".into()],
            "existing".into(),
            Some(default_rename()),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("衝突"));
        // 既存フォルダは変更されていない
        assert!(dir.join("a").exists());
        assert!(dir.join("b").exists());
        assert!(dir.join("existing").exists());
        // existing の中身は空のまま
        assert_eq!(fs::read_dir(dir.join("existing")).unwrap().count(), 0);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn execute_blocked_when_selection_invalid() {
        let dir = tempdir();
        make_dirs(&dir, &["a"]);
        let parent = dir.to_string_lossy().into_owned();
        // 選択 1 件のみ → エラー
        let result = execute_group_impl(
            parent,
            vec!["a".into()],
            "group".into(),
            Some(default_rename()),
        );
        assert!(result.is_err());
        assert!(!dir.join("group").exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn execute_no_rename_just_moves() {
        let dir = tempdir();
        make_dirs(&dir, &["a", "b"]);
        let parent = dir.to_string_lossy().into_owned();
        let record = execute_group_impl(
            parent,
            vec!["a".into(), "b".into()],
            "group".into(),
            None,
        )
        .unwrap();
        let group = dir.join("group");
        assert!(group.join("a").is_dir());
        assert!(group.join("b").is_dir());
        // CreateDir + Rename×2 = 3 ops
        assert_eq!(record.ops.len(), 3);
        fs::remove_dir_all(&dir).ok();
    }

    // ── execute_group + undo_rename round trip ───────────────────

    #[test]
    fn execute_and_undo_round_trip() {
        let dir = tempdir();
        let names = ["[A] Title 第01巻", "[A] Title 第02巻", "[A] Title 第03巻"];
        make_dirs(&dir, &names);
        let parent = dir.to_string_lossy().into_owned();

        let record = execute_group_impl(
            parent.clone(),
            names.iter().map(|s| s.to_string()).collect(),
            "[A] Title".into(),
            Some(default_rename()),
        )
        .unwrap();

        // UNDO
        let views: Vec<OpView<'_>> = record
            .ops
            .iter()
            .map(|op| match op {
                RenameOp::Rename { old_path, new_path } => OpView::Rename {
                    old_path,
                    new_path,
                },
                RenameOp::CreateDir { path } => OpView::CreateDir { path },
            })
            .collect();
        undo_ops(&views).unwrap();

        // 元の状態に戻っていること
        for name in &names {
            assert!(dir.join(name).is_dir());
        }
        assert!(!dir.join("[A] Title").exists());

        fs::remove_dir_all(&dir).ok();
    }
}
