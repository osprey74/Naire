import { useState } from "react";
import type { Macro, RenameStep } from "../../../types/rename";
import StepRow from "./StepRow";
import { initialStep, type StepKind } from "./step-defaults";
import styles from "./MacroEditor.module.css";

export interface MacroEditorProps {
  macro: Macro;
  onSave: (m: Macro) => void;
  onDelete: () => void;
  onCancel: () => void;
}

export default function MacroEditor(props: MacroEditorProps) {
  const { macro, onSave, onDelete, onCancel } = props;
  const [name, setName] = useState(macro.name);
  const [steps, setSteps] = useState<RenameStep[]>(macro.steps);

  const updateStep = (i: number, next: RenameStep) =>
    setSteps((arr) => arr.map((s, idx) => (idx === i ? next : s)));

  const removeStep = (i: number) =>
    setSteps((arr) => arr.filter((_, idx) => idx !== i));

  const moveStep = (i: number, dir: -1 | 1) =>
    setSteps((arr) => {
      const j = i + dir;
      if (j < 0 || j >= arr.length) return arr;
      const next = arr.slice();
      [next[i], next[j]] = [next[j], next[i]];
      return next;
    });

  const addStep = (kind: StepKind) => {
    setSteps((arr) => [...arr, initialStep(kind)]);
  };

  const handleSave = () => {
    const now = new Date().toISOString();
    onSave({
      ...macro,
      name: name.trim() || "(無題)",
      steps,
      updated_at: now,
    });
  };

  return (
    <div className={styles.backdrop} onClick={onCancel}>
      <div
        className={styles.modal}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby="macro-editor-title"
      >
        <div className={styles.header}>
          <h2 id="macro-editor-title" className={styles.title}>
            マクロ編集
          </h2>
          <button
            type="button"
            className={styles.closeBtn}
            onClick={onCancel}
            title="閉じる"
          >
            ×
          </button>
        </div>

        <div className={styles.body}>
          <label className={styles.nameField}>
            <span className={styles.label}>マクロ名</span>
            <input
              type="text"
              className={styles.nameInput}
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="例: 作者ソートキー追加"
              spellCheck={false}
              autoFocus
            />
          </label>

          <div className={styles.stepsSection}>
            <div className={styles.stepsHeader}>
              <span className={styles.label}>ステップ（上から順に適用）</span>
            </div>
            {steps.length === 0 ? (
              <div className={styles.empty}>
                まだステップがありません。下のボタンから追加してください。
              </div>
            ) : (
              steps.map((step, i) => (
                <StepRow
                  key={i}
                  index={i}
                  step={step}
                  onChange={(s) => updateStep(i, s)}
                  onRemove={() => removeStep(i)}
                  onMoveUp={i > 0 ? () => moveStep(i, -1) : null}
                  onMoveDown={
                    i < steps.length - 1 ? () => moveStep(i, 1) : null
                  }
                />
              ))
            )}
            <div className={styles.addRow}>
              <span className={styles.addLabel}>追加:</span>
              <button
                type="button"
                className={styles.addBtn}
                onClick={() => addStep("builtin")}
              >
                + 定型
              </button>
              <button
                type="button"
                className={styles.addBtn}
                onClick={() => addStep("regex")}
              >
                + 正規表現
              </button>
              <button
                type="button"
                className={styles.addBtn}
                onClick={() => addStep("wildcard")}
              >
                + ワイルドカード
              </button>
              <button
                type="button"
                className={styles.addBtn}
                onClick={() => addStep("char_convert")}
              >
                + 文字変換
              </button>
            </div>
          </div>
        </div>

        <div className={styles.footer}>
          <button
            type="button"
            className={`${styles.btn} ${styles.danger}`}
            onClick={onDelete}
          >
            削除
          </button>
          <div className={styles.footerRight}>
            <button type="button" className={styles.btn} onClick={onCancel}>
              キャンセル
            </button>
            <button
              type="button"
              className={`${styles.btn} ${styles.primary}`}
              onClick={handleSave}
            >
              保存
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
