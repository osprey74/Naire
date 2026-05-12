import styles from "./ActionBar.module.css";

export interface ActionBarProps {
  canRename: boolean;
  canUndo: boolean;
  undoCount: number;
  onRename: () => void;
  onUndo: () => void;
  onClear: () => void;
}

export default function ActionBar(props: ActionBarProps) {
  const { canRename, canUndo, undoCount, onRename, onUndo, onClear } = props;
  return (
    <div className={styles.bar}>
      <button
        type="button"
        className={`${styles.button} ${styles.primary}`}
        onClick={onRename}
        disabled={!canRename}
      >
        リネーム実行
      </button>
      <button
        type="button"
        className={styles.button}
        onClick={onUndo}
        disabled={!canUndo}
        title={canUndo ? `${undoCount} 件のリネームを戻せます` : "戻せる操作がありません"}
      >
        元に戻す{undoCount > 0 ? ` (${undoCount})` : ""} <span className={styles.kbd}>Ctrl+Z</span>
      </button>
      <button type="button" className={styles.button} onClick={onClear}>
        再読み込み
      </button>
    </div>
  );
}
