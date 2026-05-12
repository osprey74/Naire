import { useEffect, useRef } from "react";
import type { PreviewItem } from "../types/rename";
import styles from "./PreviewPanel.module.css";

export interface PreviewPanelProps {
  items: PreviewItem[];
  loading: boolean;
  error: string | null;
  selectedPaths: Set<string>;
  onSelectionChange: (next: Set<string>) => void;
}

export default function PreviewPanel(props: PreviewPanelProps) {
  const { items, loading, error, selectedPaths, onSelectionChange } = props;
  const anchorRef = useRef<number | null>(null);

  // items が変化したら、現在の items に存在しないパスを selectedPaths から間引く。
  // フィルタ変更やフォルダ移動で消えた行の選択を引きずらないため。
  useEffect(() => {
    if (selectedPaths.size === 0) return;
    const present = new Set(items.map((i) => i.path));
    let changed = false;
    const next = new Set<string>();
    for (const p of selectedPaths) {
      if (present.has(p)) next.add(p);
      else changed = true;
    }
    if (changed) onSelectionChange(next);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [items]);

  const handleRowClick = (index: number, e: React.MouseEvent) => {
    const path = items[index].path;
    if (e.shiftKey && anchorRef.current !== null) {
      const from = Math.min(anchorRef.current, index);
      const to = Math.max(anchorRef.current, index);
      const next = new Set<string>();
      for (let i = from; i <= to; i++) next.add(items[i].path);
      onSelectionChange(next);
    } else if (e.ctrlKey || e.metaKey) {
      const next = new Set(selectedPaths);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      onSelectionChange(next);
      anchorRef.current = index;
    } else {
      onSelectionChange(new Set([path]));
      anchorRef.current = index;
    }
  };

  const handleWrapClick = (e: React.MouseEvent) => {
    // テーブル外の空白部クリックで選択解除
    if (e.target === e.currentTarget) {
      onSelectionChange(new Set());
      anchorRef.current = null;
    }
  };

  const selectAll = () => {
    onSelectionChange(new Set(items.map((i) => i.path)));
  };

  const selectedCount = selectedPaths.size;
  const allSelected = items.length > 0 && selectedCount === items.length;

  return (
    <div className={styles.panel}>
      <div className={styles.tableWrap} onClick={handleWrapClick}>
        <table className={styles.table}>
          <thead>
            <tr>
              <th className={styles.col}>現在の名前</th>
              <th className={styles.col}>新しい名前</th>
              <th className={styles.col}>フォルダ</th>
            </tr>
          </thead>
          <tbody>
            {items.map((item, i) => {
              const selected = selectedPaths.has(item.path);
              return (
                <tr
                  key={item.path}
                  className={[
                    item.is_changed ? styles.changed : "",
                    selected ? styles.selected : "",
                  ]
                    .filter(Boolean)
                    .join(" ")}
                  onClick={(e) => handleRowClick(i, e)}
                >
                  <td className={styles.cell}>{item.original}</td>
                  <td className={styles.cell}>{item.renamed}</td>
                  <td className={styles.cell}>{item.folder}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <div className={styles.statusBar}>
        {error ? (
          <span className={styles.error}>エラー: {error}</span>
        ) : loading ? (
          <span>読み込み中...</span>
        ) : selectedCount > 0 ? (
          <span className={styles.statusGroup}>
            <span>
              {items.length} 件中 <strong>{selectedCount} 件</strong> 選択
            </span>
            {!allSelected && (
              <button
                type="button"
                className={styles.linkBtn}
                onClick={selectAll}
              >
                すべて選択
              </button>
            )}
            <button
              type="button"
              className={styles.linkBtn}
              onClick={() => onSelectionChange(new Set())}
            >
              選択解除
            </button>
          </span>
        ) : (
          <span>{items.length} 件</span>
        )}
      </div>
    </div>
  );
}
