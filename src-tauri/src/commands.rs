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
