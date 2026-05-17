import type { GroupPreview, TargetType } from "../../../types/rename";
import styles from "./GroupPanel.module.css";

export interface GroupState {
  groupName: string;
  nameDirty: boolean; // ユーザがフォルダ名を手動編集したか
  renameEnabled: boolean;
  prefix: string;
  suffix: string;
  digits: number;
  start: number;
  step: number;
}

export interface GroupPanelProps {
  target: TargetType;
  selectedCount: number;
  state: GroupState;
  onChange: (next: GroupState) => void;
  preview: GroupPreview | null;
  computing: boolean;
  onExecute: () => void;
  canExecute: boolean;
}

export default function GroupPanel(props: GroupPanelProps) {
  const {
    target,
    selectedCount,
    state,
    onChange,
    preview,
    computing,
    onExecute,
    canExecute,
  } = props;

  const set = <K extends keyof GroupState>(key: K, value: GroupState[K]) =>
    onChange({ ...state, [key]: value });

  if (target !== "folder") {
    return (
      <div className={styles.panel}>
        <div className={styles.notice}>
          集約機能は「フォルダ」モードでのみ使えます。ツールバーで切り替えてください。
        </div>
      </div>
    );
  }

  if (selectedCount < 2) {
    return (
      <div className={styles.panel}>
        <div className={styles.notice}>
          集約には <strong>2 件以上のフォルダ選択</strong> が必要です（プレビューパネルから Ctrl / Shift クリックで複数選択）。
        </div>
      </div>
    );
  }

  const commonPrefix = preview?.common_prefix ?? "";
  const errorMsg = preview?.error ?? null;
  const conflict = preview?.conflict ?? false;

  return (
    <div className={styles.panel}>
      <div className={styles.info}>
        選択フォルダ: <strong>{selectedCount} 件</strong>
        {preview?.parent_folder && (
          <>
            {" "}
            親: <span className={styles.parent}>{preview.parent_folder}</span>
          </>
        )}
      </div>

      <div className={styles.field}>
        <label className={styles.label}>集約フォルダ名:</label>
        <div className={styles.row}>
          <input
            type="text"
            className={styles.nameInput}
            value={state.groupName}
            onChange={(e) =>
              onChange({
                ...state,
                groupName: e.target.value,
                nameDirty: true,
              })
            }
            placeholder="（自動算出）"
          />
          <button
            type="button"
            className={styles.revertBtn}
            disabled={!state.nameDirty || !commonPrefix}
            title="共通プレフィックスから自動算出した値に戻す"
            onClick={() =>
              onChange({
                ...state,
                groupName: commonPrefix,
                nameDirty: false,
              })
            }
          >
            自動算出に戻す
          </button>
        </div>
      </div>

      <fieldset className={styles.renameSection}>
        <legend>
          <label className={styles.legendLabel}>
            <input
              type="checkbox"
              checked={state.renameEnabled}
              onChange={(e) => set("renameEnabled", e.target.checked)}
            />
            連番リネームを実行する
          </label>
        </legend>
        <div className={styles.renameGrid}>
          <label className={styles.smallField}>
            プレフィックス:
            <input
              type="text"
              className={styles.text}
              value={state.prefix}
              onChange={(e) => set("prefix", e.target.value)}
              disabled={!state.renameEnabled}
            />
          </label>
          <label className={styles.smallField}>
            サフィックス:
            <input
              type="text"
              className={styles.text}
              value={state.suffix}
              onChange={(e) => set("suffix", e.target.value)}
              disabled={!state.renameEnabled}
            />
          </label>
          <label className={styles.smallField}>
            桁数:
            <input
              type="number"
              min={1}
              className={styles.num}
              value={state.digits}
              onChange={(e) => set("digits", Math.max(1, Number(e.target.value) || 1))}
              disabled={!state.renameEnabled}
            />
          </label>
          <label className={styles.smallField}>
            開始:
            <input
              type="number"
              min={0}
              className={styles.num}
              value={state.start}
              onChange={(e) => set("start", Math.max(0, Number(e.target.value) || 0))}
              disabled={!state.renameEnabled}
            />
          </label>
          <label className={styles.smallField}>
            ステップ:
            <input
              type="number"
              min={1}
              className={styles.num}
              value={state.step}
              onChange={(e) => set("step", Math.max(1, Number(e.target.value) || 1))}
              disabled={!state.renameEnabled}
            />
          </label>
        </div>
        <div className={styles.hint}>
          進数は連番設定パネル（下部）の値を共有します。
        </div>
      </fieldset>

      {errorMsg && (
        <div className={styles.errorMsg}>
          {errorMsg}
        </div>
      )}
      {!errorMsg && conflict && (
        <div className={styles.warnMsg}>
          集約フォルダ名「{preview?.group_name}」は既に存在します。別名を指定してください。
        </div>
      )}

      {!errorMsg && preview && preview.items.length > 0 && (
        <div className={styles.previewSection}>
          <div className={styles.previewTitle}>
            集約後プレビュー（昇順・{preview.items.length} 件）
          </div>
          <div className={styles.previewList}>
            {preview.items.map((item) => (
              <div className={styles.previewRow} key={item.final_path}>
                <span className={styles.previewOrig}>{item.original_name}</span>
                <span className={styles.previewArrow}>→</span>
                <span className={styles.previewNew}>
                  <span className={styles.previewFolder}>
                    {preview.group_name}/
                  </span>
                  {item.renamed}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className={styles.exec}>
        <button
          type="button"
          className={styles.execBtn}
          onClick={onExecute}
          disabled={!canExecute || computing}
        >
          {computing ? "処理中…" : "集約実行"}
        </button>
      </div>
    </div>
  );
}
