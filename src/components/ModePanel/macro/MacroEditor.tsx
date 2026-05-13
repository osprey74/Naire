import { useLayoutEffect, useRef, useState } from "react";
import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  arrayMove,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { Macro, RenameStep } from "../../../types/rename";
import StepRow from "./StepRow";
import { initialStep, type StepKind } from "./step-defaults";
import styles from "./MacroEditor.module.css";

export interface MacroEditorProps {
  macro: Macro;
  onSave: (m: Macro) => void;
  onDelete: () => void;
  onCancel: () => void;
  onExport: (m: Macro) => void;
}

interface StepWithId {
  id: string;
  step: RenameStep;
}

function SortableStepRow({
  id,
  index,
  step,
  onChange,
  onRemove,
}: {
  id: string;
  index: number;
  step: RenameStep;
  onChange: (step: RenameStep) => void;
  onRemove: () => void;
}) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id });

  // dnd-kit の transform / transition は毎フレーム変動するため CSS モジュールでは表現
  // できない。useLayoutEffect で DOM へ直接適用してインライン style を回避する。
  const localRef = useRef<HTMLDivElement | null>(null);
  const setRefs = (node: HTMLDivElement | null) => {
    localRef.current = node;
    setNodeRef(node);
  };
  useLayoutEffect(() => {
    const el = localRef.current;
    if (!el) return;
    const t = CSS.Transform.toString(transform);
    if (t) el.style.transform = t;
    else el.style.removeProperty("transform");
    if (transition) el.style.transition = transition;
    else el.style.removeProperty("transition");
    if (isDragging) {
      el.style.opacity = "0.5";
      el.style.zIndex = "10";
    } else {
      el.style.removeProperty("opacity");
      el.style.removeProperty("z-index");
    }
  });

  return (
    <div ref={setRefs}>
      <StepRow
        index={index}
        step={step}
        onChange={onChange}
        onRemove={onRemove}
        dragHandleProps={{ ...attributes, ...listeners }}
      />
    </div>
  );
}

export default function MacroEditor(props: MacroEditorProps) {
  const { macro, onSave, onDelete, onCancel, onExport } = props;
  const [name, setName] = useState(macro.name);
  const [items, setItems] = useState<StepWithId[]>(() =>
    macro.steps.map((s) => ({ id: crypto.randomUUID(), step: s })),
  );

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  const updateStep = (id: string, next: RenameStep) =>
    setItems((arr) =>
      arr.map((it) => (it.id === id ? { ...it, step: next } : it)),
    );

  const removeStep = (id: string) =>
    setItems((arr) => arr.filter((it) => it.id !== id));

  const addStep = (kind: StepKind) =>
    setItems((arr) => [
      ...arr,
      { id: crypto.randomUUID(), step: initialStep(kind) },
    ]);

  const onDragEnd = (e: DragEndEvent) => {
    const { active, over } = e;
    if (!over || active.id === over.id) return;
    setItems((arr) => {
      const oldIdx = arr.findIndex((x) => x.id === active.id);
      const newIdx = arr.findIndex((x) => x.id === over.id);
      if (oldIdx < 0 || newIdx < 0) return arr;
      return arrayMove(arr, oldIdx, newIdx);
    });
  };

  const collectSteps = () => items.map((it) => it.step);

  const handleSave = () => {
    const now = new Date().toISOString();
    onSave({
      ...macro,
      name: name.trim() || "(無題)",
      steps: collectSteps(),
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
              <span className={styles.label}>
                ステップ（上から順に適用、ドラッグで並べ替え）
              </span>
            </div>
            {items.length === 0 ? (
              <div className={styles.empty}>
                まだステップがありません。下のボタンから追加してください。
              </div>
            ) : (
              <DndContext
                sensors={sensors}
                collisionDetection={closestCenter}
                onDragEnd={onDragEnd}
              >
                <SortableContext
                  items={items.map((it) => it.id)}
                  strategy={verticalListSortingStrategy}
                >
                  {items.map((it, i) => (
                    <SortableStepRow
                      key={it.id}
                      id={it.id}
                      index={i}
                      step={it.step}
                      onChange={(s) => updateStep(it.id, s)}
                      onRemove={() => removeStep(it.id)}
                    />
                  ))}
                </SortableContext>
              </DndContext>
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
          <div className={styles.footerLeft}>
            <button
              type="button"
              className={`${styles.btn} ${styles.danger}`}
              onClick={onDelete}
            >
              削除
            </button>
            <button
              type="button"
              className={styles.btn}
              onClick={() =>
                onExport({
                  ...macro,
                  name: name.trim() || "(無題)",
                  steps: collectSteps(),
                })
              }
              title="編集中のマクロを JSON ファイルへ書き出し"
            >
              ↓ エクスポート
            </button>
          </div>
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
