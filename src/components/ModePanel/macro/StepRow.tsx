import type { HTMLAttributes } from "react";
import type { BuiltinOp, RenameStep } from "../../../types/rename";
import BuiltinMenu from "../builtin/BuiltinMenu";
import BuiltinParams from "../builtin/BuiltinParams";
import { initialOp } from "../builtin/builtin-defaults";
import { initialStep, type StepKind } from "./step-defaults";
import styles from "./StepRow.module.css";

export interface StepRowProps {
  index: number;
  step: RenameStep;
  onChange: (step: RenameStep) => void;
  onRemove: () => void;
  dragHandleProps?: HTMLAttributes<HTMLButtonElement>;
}

/// MacroEditor 内で 1 ステップを編集するための行。
/// kind selector + 種別ごとのフォームを並べる。
export default function StepRow(props: StepRowProps) {
  const { index, step, onChange, onRemove, dragHandleProps } = props;

  const setKind = (kind: StepKind) => {
    if (kind !== step.kind) onChange(initialStep(kind));
  };

  return (
    <div className={styles.row}>
      <div className={styles.header}>
        <button
          type="button"
          className={styles.dragHandle}
          aria-label="ドラッグで並べ替え"
          title="ドラッグで並べ替え"
          {...dragHandleProps}
        >
          ⋮⋮
        </button>
        <span className={styles.idx}>Step {index + 1}</span>
        <select
          aria-label="ステップ種別"
          className={styles.kindSelect}
          value={step.kind}
          onChange={(e) => setKind(e.target.value as StepKind)}
        >
          <option value="builtin">定型</option>
          <option value="regex">正規表現</option>
          <option value="wildcard">ワイルドカード</option>
          <option value="char_convert">文字変換</option>
        </select>
        <div className={styles.controls}>
          <button
            type="button"
            className={styles.removeBtn}
            onClick={onRemove}
            title="削除"
          >
            ×
          </button>
        </div>
      </div>

      <div className={styles.body}>
        {step.kind === "regex" && (
          <>
            <SimpleInput
              label="検索"
              value={step.search}
              onChange={(v) => onChange({ ...step, search: v })}
              placeholder="正規表現パターン"
            />
            <SimpleInput
              label="置換"
              value={step.replace}
              onChange={(v) => onChange({ ...step, replace: v })}
              placeholder={String.raw`\1 \t \e \orig など`}
            />
          </>
        )}
        {step.kind === "wildcard" && (
          <>
            <SimpleInput
              label="検索（ワイルドカード）"
              value={step.search}
              onChange={(v) => onChange({ ...step, search: v })}
              placeholder="例: *.jpg"
            />
            <SimpleInput
              label="置換"
              value={step.replace}
              onChange={(v) => onChange({ ...step, replace: v })}
              placeholder={String.raw`\t.png など`}
            />
          </>
        )}
        {step.kind === "char_convert" && (
          <>
            <SimpleInput
              label="変換前 (from)"
              value={step.from}
              onChange={(v) => onChange({ ...step, from: v })}
              placeholder="a-z または 0,i,ii,..."
            />
            <SimpleInput
              label="変換後 (to)"
              value={step.to}
              onChange={(v) => onChange({ ...step, to: v })}
              placeholder="A-Z または 0,1,2,..."
            />
          </>
        )}
        {step.kind === "builtin" && (
          <BuiltinEmbeddedEditor
            op={step.op}
            onChange={(op) => onChange({ ...step, op })}
          />
        )}
      </div>
    </div>
  );
}

function SimpleInput(props: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
}) {
  return (
    <label className={styles.field}>
      <span className={styles.label}>{props.label}</span>
      <input
        type="text"
        className={styles.input}
        value={props.value}
        onChange={(e) => props.onChange(e.target.value)}
        placeholder={props.placeholder}
        spellCheck={false}
      />
    </label>
  );
}

function BuiltinEmbeddedEditor(props: {
  op: BuiltinOp;
  onChange: (op: BuiltinOp) => void;
}) {
  return (
    <div className={styles.builtinEmbed}>
      <BuiltinMenu
        selectedKind={props.op.type}
        onSelect={(kind) => props.onChange(initialOp(kind))}
      />
      <BuiltinParams op={props.op} onChange={props.onChange} />
    </div>
  );
}
