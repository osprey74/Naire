import type { Macro } from "../../../types/rename";
import { describeStep } from "./step-defaults";
import styles from "./MacroPanel.module.css";

export interface MacroPanelProps {
  macros: Macro[];
  currentMacroId: string | null;
  stepIndex: number;
  hasItems: boolean;
  onSelect: (id: string | null) => void;
  onCreateNew: () => void;
  onEdit: (id: string) => void;
  onStepForward: () => void;
  onApplyAll: () => void;
  onReset: () => void;
}

export default function MacroPanel(props: MacroPanelProps) {
  const {
    macros,
    currentMacroId,
    stepIndex,
    hasItems,
    onSelect,
    onCreateNew,
    onEdit,
    onStepForward,
    onApplyAll,
    onReset,
  } = props;

  const current = macros.find((m) => m.id === currentMacroId) ?? null;
  const totalSteps = current?.steps.length ?? 0;
  const canStep = !!current && stepIndex < totalSteps && hasItems;
  const canReset = !!current && stepIndex > 0;
  const running = stepIndex > 0; // 進行中ならドロップダウンをロック
  const currentStep =
    current && stepIndex < totalSteps ? current.steps[stepIndex] : null;

  return (
    <div className={styles.panel}>
      <div className={styles.selectorRow}>
        <select
          aria-label="マクロを選択"
          className={styles.select}
          value={currentMacroId ?? ""}
          onChange={(e) => onSelect(e.target.value || null)}
          disabled={running}
        >
          <option value="">— マクロを選択 —</option>
          {macros.map((m) => (
            <option key={m.id} value={m.id}>
              {m.name}（{m.steps.length} ステップ）
            </option>
          ))}
        </select>
        <button type="button" className={styles.btn} onClick={onCreateNew}>
          新規
        </button>
        <button
          type="button"
          className={styles.btn}
          onClick={() => current && onEdit(current.id)}
          disabled={!current || running}
        >
          編集
        </button>
      </div>

      {current ? (
        <>
          <div className={styles.progress}>
            ステップ進行: <strong>{stepIndex}</strong> / {totalSteps}
          </div>
          {currentStep ? (
            <div className={styles.currentStep}>
              次のステップ: <code>{describeStep(currentStep)}</code>
            </div>
          ) : (
            <div className={styles.currentStep}>
              <strong>全ステップ適用済み</strong> — [リネーム実行] でディスクに反映
            </div>
          )}
          <div className={styles.controlsRow}>
            <button
              type="button"
              className={styles.controlBtn}
              onClick={onReset}
              disabled={!canReset}
              title="マクロ開始前に巻き戻す"
            >
              ◀戻す
            </button>
            <button
              type="button"
              className={`${styles.controlBtn} ${styles.primary}`}
              onClick={onStepForward}
              disabled={!canStep}
            >
              ステップ処理▶
            </button>
            <button
              type="button"
              className={styles.controlBtn}
              onClick={onApplyAll}
              disabled={!canStep}
            >
              すべて適用
            </button>
          </div>
        </>
      ) : (
        <div className={styles.placeholder}>
          マクロを選択するか「新規」で作成してください。
        </div>
      )}
    </div>
  );
}
