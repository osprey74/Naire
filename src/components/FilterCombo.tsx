import { useEffect, useRef, useState } from "react";
import styles from "./FilterCombo.module.css";

interface FilterPreset {
  label: string;
  value: string;
}

export const FILTER_PRESETS: FilterPreset[] = [
  { label: "すべて", value: "*" },
  {
    label: "画像",
    value:
      "*.bmp;*.dib;*.gif;*.jpg;*.jpeg;*.jpe;*.png;*.ico;*.cur;*.webp;*.heic;*.heif",
  },
  {
    label: "動画",
    value: "*.mp4;*.mov;*.avi;*.mkv;*.wmv;*.flv;*.webm;*.m4v",
  },
  {
    label: "音楽",
    value: "*.mp3;*.wav;*.flac;*.aac;*.m4a;*.ogg;*.wma",
  },
  {
    label: "ドキュメント",
    value: "*.txt;*.doc;*.docx;*.xls;*.xlsx;*.ppt;*.pptx;*.pdf;*.md;*.rtf",
  },
  {
    label: "アーカイブ",
    value: "*.zip;*.rar;*.7z;*.tar;*.gz;*.bz2;*.xz",
  },
  { label: "隠しファイル除外", value: "^(.*)" },
];

export interface FilterComboProps {
  value: string;
  history: string[];
  onChange: (v: string) => void;
  onCommit: (v: string) => void;
}

export default function FilterCombo(props: FilterComboProps) {
  const { value, history, onChange, onCommit } = props;
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDocMouseDown = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onDocMouseDown);
    return () => document.removeEventListener("mousedown", onDocMouseDown);
  }, [open]);

  const choose = (v: string) => {
    onChange(v);
    onCommit(v);
    setOpen(false);
  };

  return (
    <div ref={containerRef} className={styles.combo}>
      <input
        type="text"
        className={styles.input}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            onCommit(value);
            setOpen(false);
          } else if (e.key === "Escape") {
            setOpen(false);
          } else if (e.key === "ArrowDown" && !open) {
            e.preventDefault();
            setOpen(true);
          }
        }}
        placeholder="*"
        spellCheck={false}
      />
      <button
        type="button"
        className={styles.toggle}
        onClick={() => setOpen((v) => !v)}
        aria-label="フィルタプリセット"
        tabIndex={-1}
      >
        ▼
      </button>
      {open && (
        <div className={styles.menu} role="listbox">
          <div className={styles.menuLabel}>プリセット</div>
          {FILTER_PRESETS.map((p) => (
            <button
              key={p.label}
              type="button"
              className={styles.menuItem}
              onClick={() => choose(p.value)}
            >
              <span className={styles.itemLabel}>{p.label}</span>
              <span className={styles.itemHint}>{p.value}</span>
            </button>
          ))}
          {history.length > 0 && (
            <>
              <div className={styles.divider} />
              <div className={styles.menuLabel}>履歴</div>
              {history.map((h, i) => (
                <button
                  key={`h-${i}`}
                  type="button"
                  className={styles.menuItem}
                  onClick={() => choose(h)}
                >
                  <span className={styles.itemHint}>{h}</span>
                </button>
              ))}
            </>
          )}
        </div>
      )}
    </div>
  );
}

/// `*` 既定値や空文字は履歴に含めず、重複は前方に詰めて 10 件で打ち切る。
export function pushHistory(history: string[], raw: string): string[] {
  const trimmed = raw.trim();
  if (!trimmed) return history;
  const without = history.filter((x) => x !== trimmed);
  return [trimmed, ...without].slice(0, 10);
}
