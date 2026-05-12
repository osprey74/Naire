use std::path::Path;

/// 1 件の RenameRecord を構成する op の参照ビュー。
/// `commands.rs` の `RenameOp` から `(old_path, new_path)` 文字列だけ取り出す。
pub struct OpView<'a> {
    pub old_path: &'a str,
    pub new_path: &'a str,
}

/// `ops` を逆順（末尾の op から先頭の op の順）で `new_path → old_path` に
/// リネームし直す。事前に全 op の前提条件（new_path 存在 / old_path 空き）を
/// 検証して、満たさなければ 1 件も実行しない。
///
/// 途中で `std::fs::rename` が失敗した場合は、部分的に戻った状態で `Err` を返す。
/// 呼び出し側はスタックから該当 record を捨てない選択ができる。
pub fn undo_ops(ops: &[OpView<'_>]) -> Result<(), String> {
    if ops.is_empty() {
        return Ok(());
    }

    for op in ops.iter().rev() {
        if !Path::new(op.new_path).exists() {
            return Err(format!("ファイルが見つかりません: {}", op.new_path));
        }
        if op.old_path != op.new_path && Path::new(op.old_path).exists() {
            return Err(format!("元のパスに既にファイルがあります: {}", op.old_path));
        }
    }

    let mut undone = 0usize;
    for op in ops.iter().rev() {
        if op.old_path == op.new_path {
            undone += 1;
            continue;
        }
        std::fs::rename(op.new_path, op.old_path).map_err(|e| {
            format!(
                "UNDO 失敗 {} → {}: {} （これまでに {} 件戻し済み）",
                op.new_path, op.old_path, e, undone
            )
        })?;
        undone += 1;
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
        undo_ops(&[OpView {
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
        let err = undo_ops(&[OpView {
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
        let err = undo_ops(&[OpView {
            old_path: &old_str,
            new_path: &new_str,
        }])
        .unwrap_err();
        assert!(err.contains("既にファイル"));
        fs::remove_dir_all(&dir).ok();
    }
}
