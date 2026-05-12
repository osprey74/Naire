import type { TargetType } from "../types/rename";
import FilterCombo from "./FilterCombo";
import styles from "./Toolbar.module.css";

export interface ToolbarProps {
  target: TargetType;
  onTargetChange: (t: TargetType) => void;
  recursive: boolean;
  onRecursiveChange: (v: boolean) => void;
  depth: number;
  onDepthChange: (n: number) => void;
  filter: string;
  onFilterChange: (s: string) => void;
  filterHistory: string[];
  onFilterCommit: (s: string) => void;
  onAboutClick: () => void;
}

export default function Toolbar(props: ToolbarProps) {
  const {
    target,
    onTargetChange,
    recursive,
    onRecursiveChange,
    depth,
    onDepthChange,
    filter,
    onFilterChange,
    filterHistory,
    onFilterCommit,
    onAboutClick,
  } = props;

  return (
    <div className={styles.toolbar}>
      <label className={styles.radio}>
        <input
          type="radio"
          name="target"
          value="file"
          checked={target === "file"}
          onChange={() => onTargetChange("file")}
        />
        ファイル
      </label>
      <label className={styles.radio}>
        <input
          type="radio"
          name="target"
          value="folder"
          checked={target === "folder"}
          onChange={() => onTargetChange("folder")}
        />
        フォルダ
      </label>

      <span className={styles.sep} />

      <span className={styles.field}>
        フィルタ:
        <FilterCombo
          value={filter}
          history={filterHistory}
          onChange={onFilterChange}
          onCommit={onFilterCommit}
        />
      </span>

      <span className={styles.sep} />

      <label className={styles.check}>
        <input
          type="checkbox"
          checked={recursive}
          onChange={(e) => onRecursiveChange(e.target.checked)}
        />
        サブフォルダ再帰
      </label>

      <label className={styles.field}>
        深さ:
        <input
          type="number"
          className={styles.depthInput}
          min={0}
          value={depth}
          disabled={!recursive}
          onChange={(e) => onDepthChange(Math.max(0, Number(e.target.value) || 0))}
        />
      </label>

      <button
        type="button"
        className={styles.aboutBtn}
        onClick={onAboutClick}
        title="アプリについて"
      >
        アプリについて
      </button>
    </div>
  );
}
