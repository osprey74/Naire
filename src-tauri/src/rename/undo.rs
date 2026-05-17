use std::path::Path;

/// 1 件の RenameRecord を構成する op の参照ビュー。
/// `commands.rs` の `RenameOp` から借用したパス文字列を保持する。
///
/// Phase 14 で enum 化:
/// - `Rename`: 既存のリネーム / 移動の逆適用（`new_path → old_path`）
/// - `CreateDir`: フォルダ集約時に作成された親フォルダの逆適用（空フォルダの削除）
pub enum OpView<'a> {
    Rename {
        old_path: &'a str,
        new_path: &'a str,
    },
    CreateDir {
        path: &'a str,
    },
}

/// `ops` を逆順（末尾の op から先頭の op の順）で逆適用する。
/// 事前に全 op の前提条件を検証して、満たさなければ 1 件も実行しない。
///
/// 検証条件:
/// - `Rename`: `new_path` が存在し、`old_path` が空いている（同パスは除外）
/// - `CreateDir`: `path` が存在し、ディレクトリである
///
/// 途中で `std::fs::rename` / `remove_dir` が失敗した場合は、
/// 部分的に戻った状態で `Err` を返す。呼び出し側はスタックから該当 record を
/// 捨てない選択ができる。
///
/// 空フォルダでない `CreateDir` の `remove_dir` は OS 側で失敗するため、
/// メッセージはそのまま伝播する。
pub fn undo_ops(ops: &[OpView<'_>]) -> Result<(), String> {
    if ops.is_empty() {
        return Ok(());
    }

    // 事前検証（全 op のチェックを通過してから実行に入る）
    for op in ops.iter().rev() {
        match op {
            OpView::Rename { old_path, new_path } => {
                if !Path::new(new_path).exists() {
                    return Err(format!("ファイルが見つかりません: {}", new_path));
                }
                if old_path != new_path && Path::new(old_path).exists() {
                    return Err(format!("元のパスに既にファイルがあります: {}", old_path));
                }
            }
            OpView::CreateDir { path } => {
                let p = Path::new(path);
                if !p.exists() {
                    return Err(format!("フォルダが見つかりません: {}", path));
                }
                if !p.is_dir() {
                    return Err(format!("ディレクトリではありません: {}", path));
                }
            }
        }
    }

    let mut undone = 0usize;
    for op in ops.iter().rev() {
        match op {
            OpView::Rename { old_path, new_path } => {
                if old_path == new_path {
                    undone += 1;
                    continue;
                }
                std::fs::rename(new_path, old_path).map_err(|e| {
                    format!(
                        "UNDO 失敗 {} → {}: {} （これまでに {} 件戻し済み）",
                        new_path, old_path, e, undone
                    )
                })?;
                undone += 1;
            }
            OpView::CreateDir { path } => {
                std::fs::remove_dir(path).map_err(|e| {
                    format!(
                        "UNDO 失敗 フォルダ削除 {}: {} （これまでに {} 件戻し済み）",
                        path, e, undone
                    )
                })?;
                undone += 1;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn tempdir() -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "naire-undo-test-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn undo_reverses_rename() {
        let dir = tempdir();
        let old_path = dir.join("a.txt");
        let new_path = dir.join("b.txt");
        fs::write(&old_path, "hello").unwrap();
        fs::rename(&old_path, &new_path).unwrap();
        assert!(!old_path.exists() && new_path.exists());

        let old_str = old_path.to_string_lossy().into_owned();
        let new_str = new_path.to_string_lossy().into_owned();
        undo_ops(&[OpView::Rename {
            old_path: &old_str,
            new_path: &new_str,
        }])
        .unwrap();

        assert!(old_path.exists() && !new_path.exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn undo_fails_when_new_path_missing() {
        let dir = tempdir();
        let old_str = dir.join("a.txt").to_string_lossy().into_owned();
        let new_str = dir.join("b.txt").to_string_lossy().into_owned();
        // b.txt 不在
        let err = undo_ops(&[OpView::Rename {
            old_path: &old_str,
            new_path: &new_str,
        }])
        .unwrap_err();
        assert!(err.contains("見つかりません"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn undo_fails_when_old_path_taken() {
        let dir = tempdir();
        let old_path = dir.join("a.txt");
        let new_path = dir.join("b.txt");
        // 両方存在 → old_path が空いていない
        fs::write(&old_path, "x").unwrap();
        fs::write(&new_path, "y").unwrap();
        let old_str = old_path.to_string_lossy().into_owned();
        let new_str = new_path.to_string_lossy().into_owned();
        let err = undo_ops(&[OpView::Rename {
            old_path: &old_str,
            new_path: &new_str,
        }])
        .unwrap_err();
        assert!(err.contains("既にファイル"));
        fs::remove_dir_all(&dir).ok();
    }

    // Phase 14.2: CreateDir 逆適用テスト

    #[test]
    fn undo_create_dir_removes_empty_dir() {
        let dir = tempdir();
        let group_dir = dir.join("group");
        fs::create_dir(&group_dir).unwrap();
        assert!(group_dir.exists());

        let group_str = group_dir.to_string_lossy().into_owned();
        undo_ops(&[OpView::CreateDir { path: &group_str }]).unwrap();

        assert!(!group_dir.exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn undo_create_dir_fails_when_not_empty() {
        let dir = tempdir();
        let group_dir = dir.join("group");
        fs::create_dir(&group_dir).unwrap();
        fs::write(group_dir.join("leftover.txt"), "data").unwrap();

        let group_str = group_dir.to_string_lossy().into_owned();
        let err = undo_ops(&[OpView::CreateDir { path: &group_str }]).unwrap_err();
        // OS のエラーメッセージはプラットフォーム依存だが、UNDO 失敗 prefix は共通
        assert!(err.contains("UNDO 失敗"));
        assert!(group_dir.exists()); // 失敗時は残る
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn undo_create_dir_fails_when_path_missing() {
        let dir = tempdir();
        let group_str = dir.join("missing").to_string_lossy().into_owned();

        let err = undo_ops(&[OpView::CreateDir { path: &group_str }]).unwrap_err();
        assert!(err.contains("見つかりません"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn undo_mixed_record_reverses_in_reverse_order() {
        // 集約操作を模擬: CreateDir + Rename×2 を作って逆順で巻き戻す
        let dir = tempdir();
        let group_dir = dir.join("group");
        let a_orig = dir.join("a.txt");
        let b_orig = dir.join("b.txt");
        fs::write(&a_orig, "a").unwrap();
        fs::write(&b_orig, "b").unwrap();
        // 集約処理を実行: mkdir + move
        fs::create_dir(&group_dir).unwrap();
        let a_moved = group_dir.join("a.txt");
        let b_moved = group_dir.join("b.txt");
        fs::rename(&a_orig, &a_moved).unwrap();
        fs::rename(&b_orig, &b_moved).unwrap();
        assert!(group_dir.exists() && a_moved.exists() && b_moved.exists());

        let group_str = group_dir.to_string_lossy().into_owned();
        let a_orig_s = a_orig.to_string_lossy().into_owned();
        let a_moved_s = a_moved.to_string_lossy().into_owned();
        let b_orig_s = b_orig.to_string_lossy().into_owned();
        let b_moved_s = b_moved.to_string_lossy().into_owned();

        // record: [CreateDir, Rename(a→a_in_group), Rename(b→b_in_group)]
        // 逆順実行: b を戻す → a を戻す → group を削除
        undo_ops(&[
            OpView::CreateDir { path: &group_str },
            OpView::Rename {
                old_path: &a_orig_s,
                new_path: &a_moved_s,
            },
            OpView::Rename {
                old_path: &b_orig_s,
                new_path: &b_moved_s,
            },
        ])
        .unwrap();

        assert!(!group_dir.exists());
        assert!(a_orig.exists() && b_orig.exists());
        fs::remove_dir_all(&dir).ok();
    }
}
