import { open } from "@tauri-apps/plugin-dialog";
import styles from "./FolderTree.module.css";

export interface FolderTreeProps {
  folder: string;
  onFolderChange: (path: string) => void;
}

function parentOf(path: string): string | null {
  if (!path) return null;
  const norm = path.replace(/\\/g, "/").replace(/\/+$/, "");
  const idx = norm.lastIndexOf("/");
  if (idx <= 0) return null;
  const parent = norm.slice(0, idx);
  // Windows drive root e.g. "C:" → keep as "C:/"
  if (/^[A-Za-z]:$/.test(parent)) return parent + "/";
  return parent;
}

export default function FolderTree(props: FolderTreeProps) {
  const { folder, onFolderChange } = props;

  const pick = async () => {
    const picked = await open({
      directory: true,
      multiple: false,
      defaultPath: folder || undefined,
    });
    if (typeof picked === "string") {
      onFolderChange(picked);
    }
  };

  const goUp = () => {
    const parent = parentOf(folder);
    if (parent) onFolderChange(parent);
  };

  return (
    <div className={styles.tree}>
      <div className={styles.actions}>
        <button type="button" onClick={pick} className={styles.button}>
          📁 フォルダを選択
        </button>
        <button
          type="button"
          onClick={goUp}
          disabled={!folder || parentOf(folder) === null}
          className={styles.button}
          title="親フォルダへ"
        >
          ↑
        </button>
      </div>
      <div className={styles.path} title={folder}>
        {folder || <span className={styles.placeholder}>未選択</span>}
      </div>
    </div>
  );
}
