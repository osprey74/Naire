import type { SequenceConfig } from "../types/rename";
import styles from "./SequencePanel.module.css";

export interface SequencePanelProps {
  seq: SequenceConfig;
  onSeqChange: (next: SequenceConfig) => void;
}

export default function SequencePanel(props: SequencePanelProps) {
  const { seq, onSeqChange } = props;
  const set = <K extends keyof SequenceConfig>(key: K, value: SequenceConfig[K]) =>
    onSeqChange({ ...seq, [key]: value });

  return (
    <div className={styles.panel}>
      <div className={styles.title}>連番設定</div>
      <div className={styles.row}>
        <label className={styles.field}>
          開始:
          <input
            type="number"
            min={0}
            className={styles.num}
            value={seq.start}
            onChange={(e) => set("start", Math.max(0, Number(e.target.value) || 0))}
          />
        </label>
        <label className={styles.field}>
          ステップ:
          <input
            type="number"
            min={1}
            className={styles.num}
            value={seq.step}
            onChange={(e) => set("step", Math.max(1, Number(e.target.value) || 1))}
          />
        </label>
        <label className={styles.field}>
          進数:
          <select
            className={styles.select}
            value={seq.numbering}
            onChange={(e) =>
              set("numbering", e.target.value as SequenceConfig["numbering"])
            }
          >
            <option value="decimal">10 進</option>
            <option value="hex">16 進</option>
            <option value="alpha">英大文字</option>
          </select>
        </label>
      </div>
      <label className={styles.check}>
        <input
          type="checkbox"
          checked={seq.reset_per_folder}
          onChange={(e) => set("reset_per_folder", e.target.checked)}
        />
        フォルダごとにリセット
      </label>
    </div>
  );
}
