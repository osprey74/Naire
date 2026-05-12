import { useEffect, useRef } from "react";
import type { PreviewColumnWidths, PreviewItem } from "../types/rename";
import styles from "./PreviewPanel.module.css";

export interface PreviewPanelProps {
  items: PreviewItem[];
  loading: boolean;
  error: string | null;
  selectedPaths: Set<string>;
  onSelectionChange: (next: Set<string>) => void;
  columnWidths: PreviewColumnWidths;
  onColumnWidthsChange: (next: PreviewColumnWidths) => void;
}

const MIN_COL_WIDTH = 60;

export default function PreviewPanel(props: PreviewPanelProps) {
  const {
    items,
    loading,
    error,
    selectedPaths,
    onSelectionChange,
    columnWidths,
    onColumnWidthsChange,
  } = props;
  const anchorRef = useRef<number | null>(null);

  // items が変化したら、現在の items に存在しないパスを selectedPaths から間引く。
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
    if (e.target === e.currentTarget) {
      onSelectionChange(new Set());
      anchorRef.current = null;
    }
  };

  const selectAll = () => {
    onSelectionChange(new Set(items.map((i) => i.path)));
  };

  // ── カラムリサイズ ────────────────────────────────────────
  const startResize = (col: keyof PreviewColumnWidths, e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const startX = e.clientX;
    const startWidth = columnWidths[col];

    const onMove = (ev: MouseEvent) => {
      const delta = ev.clientX - startX;
      const newWidth = Math.max(MIN_COL_WIDTH, startWidth + delta);
      onColumnWidthsChange({ ...columnWidths, [col]: newWidth });
    };
    const onUp = () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  };

  const selectedCount = selectedPaths.size;
  const allSelected = items.length > 0 && selectedCount === items.length;

  return (
    <div className={styles.panel}>
      <div className={styles.tableWrap} onClick={handleWrapClick}>
        <table className={styles.table}>
          <colgroup>
            {/* 動的なカラム幅のため inline style を使う（リサイズ機能の本質） */}
            {/* eslint-disable-next-line react/forbid-dom-props */}
            <col style={{ width: `${columnWidths.original}px` }} />
            {/* eslint-disable-next-line react/forbid-dom-props */}
            <col style={{ width: `${columnWidths.renamed}px` }} />
            <col />
          </colgroup>
          <thead>
            <tr>
              <th className={styles.col}>
                <span className={styles.colLabel}>現在の名前</span>
                <span
                  className={styles.resizer}
                  onMouseDown={(e) => startResize("original", e)}
                  role="separator"
                  aria-label="現在の名前カラムの幅を調整"
                />
              </th>
              <th className={styles.col}>
                <span className={styles.colLabel}>新しい名前</span>
                <span
                  className={styles.resizer}
                  onMouseDown={(e) => startResize("renamed", e)}
                  role="separator"
                  aria-label="新しい名前カラムの幅を調整"
                />
              </th>
              <th className={styles.col}>
                <span className={styles.colLabel}>フォルダ</span>
              </th>
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
