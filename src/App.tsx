import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Toolbar from "./components/Toolbar";
import FolderTree from "./components/FolderTree";
import PreviewPanel from "./components/PreviewPanel";
import ActionBar from "./components/ActionBar";
import ModePanel, {
  type AdvancedTab,
  type Mode,
} from "./components/ModePanel/ModePanel";
import SequencePanel from "./components/SequencePanel";
import MacroEditor from "./components/ModePanel/macro/MacroEditor";
import { usePreview } from "./hooks/usePreview";
import { pushHistory } from "./components/FilterCombo";
import {
  initialOp,
  type BuiltinKind,
} from "./components/ModePanel/builtin/builtin-defaults";
import type {
  BuiltinOp,
  Macro,
  PreviewItem,
  RenameRecord,
  RenameStep,
  SequenceConfig,
  StepItem,
  TargetType,
} from "./types/rename";
import styles from "./App.module.css";

const DEFAULT_SEQ: SequenceConfig = {
  start: 0,
  step: 1,
  reset_per_folder: true,
  numbering: "decimal",
};

type Notice = { kind: "success" | "error"; message: string };

export default function App() {
  const [folder, setFolder] = useState<string>("");
  const [target, setTarget] = useState<TargetType>("file");
  const [recursive, setRecursive] = useState(false);
  const [depth, setDepth] = useState(0);
  const [filter, setFilter] = useState("*");
  const [filterHistory, setFilterHistory] = useState<string[]>([]);
  const [mode, setMode] = useState<Mode>("advanced");
  const [advancedTab, setAdvancedTab] = useState<AdvancedTab>("regex");
  const [seq, setSeq] = useState<SequenceConfig>(DEFAULT_SEQ);
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const [regexSearch, setRegexSearch] = useState("");
  const [regexReplace, setRegexReplace] = useState("");
  const [wildcardSearch, setWildcardSearch] = useState("");
  const [wildcardReplace, setWildcardReplace] = useState("");
  const [charFrom, setCharFrom] = useState("");
  const [charTo, setCharTo] = useState("");
  const [builtinOp, setBuiltinOp] = useState<BuiltinOp | null>(null);
  // ── マクロ関連の state ──────────────────────────────────────
  const [macros, setMacros] = useState<Macro[]>([]);
  const [currentMacroId, setCurrentMacroId] = useState<string | null>(null);
  const [editingMacro, setEditingMacro] = useState<Macro | null>(null);
  const [macroItems, setMacroItems] = useState<StepItem[] | null>(null);
  const [macroStepIndex, setMacroStepIndex] = useState(0);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [executing, setExecuting] = useState(false);
  // UNDO スタック。スタック単位は execute_rename 1 回分の RenameRecord。
  // 最大 20 件保持し、超えた分は古いものから捨てる（HANDOFF 仕様）。永続化しない。
  const [undoStack, setUndoStack] = useState<RenameRecord[]>([]);
  // 直近のプレビュー結果を別 state に複製しておき、selectedIndexes の派生元として
  // 使う。usePreview の戻り値を直接派生元にすると、selectedIndexes 変更 → 再フェッチ
  // → 新 items → 同じインデックスに収束、というサイクルが回るが、その過程で
  // usePreview の depKey が変化し続けることはなく安定する。
  const [lastItems, setLastItems] = useState<PreviewItem[]>([]);

  const steps = useMemo<RenameStep[]>(() => {
    if (mode === "advanced" && advancedTab === "regex" && regexSearch) {
      return [{ kind: "regex", search: regexSearch, replace: regexReplace }];
    }
    if (mode === "advanced" && advancedTab === "wildcard" && wildcardSearch) {
      return [
        { kind: "wildcard", search: wildcardSearch, replace: wildcardReplace },
      ];
    }
    if (mode === "advanced" && advancedTab === "char_convert" && charFrom) {
      return [{ kind: "char_convert", from: charFrom, to: charTo }];
    }
    if (mode === "builtin" && builtinOp) {
      return [{ kind: "builtin", op: builtinOp }];
    }
    return [];
  }, [
    mode,
    advancedTab,
    regexSearch,
    regexReplace,
    wildcardSearch,
    wildcardReplace,
    charFrom,
    charTo,
    builtinOp,
  ]);

  // selectedPaths を index に解決。最新の items（= lastItems）に基づく。
  const selectedIndexes = useMemo(() => {
    const out: number[] = [];
    lastItems.forEach((item, i) => {
      if (selectedPaths.has(item.path)) out.push(i);
    });
    return out;
  }, [lastItems, selectedPaths]);

  const preview = usePreview({
    folder,
    steps,
    target,
    recursive,
    depth,
    filter,
    seq,
    selectedIndexes,
  });

  // preview.items が変わったら lastItems に取り込む。次回の selectedIndexes
  // 再計算は新しい items 基準で行われる。
  if (preview.items !== lastItems) {
    // setState in render — React 18 以降は安全。preview.items のレファレンスが
    // 変わった瞬間に lastItems を同期させて、useMemo を再評価させる。
    setLastItems(preview.items);
  }

  // ── マクロ実行モードの導出 ──────────────────────────────────
  // macroItems を PreviewItem に変換して PreviewPanel に渡す。
  const macroAsPreview = useMemo<PreviewItem[]>(() => {
    if (!macroItems) return [];
    return macroItems.map((item) => ({
      original: item.original_name,
      renamed: item.current_name,
      folder: item.folder,
      path: item.path,
      is_changed: item.current_name !== item.original_name,
    }));
  }, [macroItems]);

  const isMacroMode = mode === "macro";
  const currentMacro = isMacroMode
    ? macros.find((m) => m.id === currentMacroId) ?? null
    : null;

  // 表示する items: マクロモード中で macroItems があればそれ、それ以外は preview
  const displayItems = isMacroMode && macroItems ? macroAsPreview : preview.items;
  const displayLoading = isMacroMode ? executing : preview.loading;
  const displayError = isMacroMode ? null : preview.error;

  const hasChanges = displayItems.some((i) => i.is_changed);
  const canRename = !!folder && hasChanges && !displayLoading && !executing;
  const canUndo = undoStack.length > 0 && !executing;

  const onRename = async () => {
    if (!canRename) return;
    setExecuting(true);
    try {
      let record: RenameRecord;
      if (isMacroMode && macroItems) {
        // マクロモード: 適用済みステップ結果をディスクへ
        const items = macroItems.map((m) => ({
          path: m.path,
          new_name: m.current_name,
        }));
        record = await invoke<RenameRecord>("apply_rename_to_filesystem", {
          items,
        });
      } else {
        record = await invoke<RenameRecord>("execute_rename", {
          folder,
          steps,
          target,
          recursive,
          depth,
          filter,
          seq,
          selectedIndexes,
        });
      }
      setUndoStack((stack) => [...stack, record].slice(-20));
      setNotice({
        kind: "success",
        message: `${record.ops.length} 件をリネームしました`,
      });
      setFilterHistory((prev) => pushHistory(prev, filter));
      setSelectedPaths(new Set());
      if (isMacroMode) {
        setMacroItems(null);
        setMacroStepIndex(0);
      }
      preview.reload();
    } catch (e) {
      setNotice({ kind: "error", message: String(e) });
    } finally {
      setExecuting(false);
    }
  };

  const onUndo = async () => {
    if (undoStack.length === 0 || executing) return;
    const last = undoStack[undoStack.length - 1];
    setExecuting(true);
    try {
      await invoke<null>("undo_rename", { record: last });
      setUndoStack((stack) => stack.slice(0, -1));
      setNotice({
        kind: "success",
        message: `${last.ops.length} 件を元に戻しました`,
      });
      setSelectedPaths(new Set());
      preview.reload();
    } catch (e) {
      // 失敗時はスタックに残す（再試行可能）
      setNotice({ kind: "error", message: String(e) });
    } finally {
      setExecuting(false);
    }
  };

  // Ctrl+Z / Cmd+Z で UNDO。INPUT/TEXTAREA フォーカス中はネイティブの
  // 編集 UNDO を尊重するため傍受しない。
  const onUndoRef = useRef(onUndo);
  onUndoRef.current = onUndo;
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.shiftKey || e.altKey) return;
      if (e.key !== "z" && e.key !== "Z") return;
      const tag = (e.target as HTMLElement | null)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;
      e.preventDefault();
      onUndoRef.current();
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, []);

  // ── マクロハンドラ ──────────────────────────────────────────
  // フォルダ/フィルタ条件が変わったらマクロ実行状態をリセット。
  useEffect(() => {
    setMacroItems(null);
    setMacroStepIndex(0);
  }, [folder, target, recursive, depth, filter]);

  // モード切替時もマクロ実行状態をリセット。
  useEffect(() => {
    if (mode !== "macro") {
      setMacroItems(null);
      setMacroStepIndex(0);
    }
  }, [mode]);

  const initMacroItems = async (): Promise<StepItem[] | null> => {
    if (!folder) {
      setNotice({ kind: "error", message: "フォルダを選択してください" });
      return null;
    }
    try {
      const items = await invoke<StepItem[]>("init_macro_items", {
        folder,
        target,
        recursive,
        depth,
        filter,
      });
      return items;
    } catch (e) {
      setNotice({ kind: "error", message: String(e) });
      return null;
    }
  };

  const applyOneStep = async (
    items: StepItem[],
    step: RenameStep,
  ): Promise<StepItem[] | null> => {
    // 選択行のみに適用するため、items に対する selectedIndexes を計算
    const selIdx: number[] = [];
    items.forEach((item, i) => {
      if (selectedPaths.has(item.path)) selIdx.push(i);
    });
    try {
      const newNames = await invoke<string[]>("apply_macro_step", {
        items,
        step,
        seq,
        selectedIndexes: selIdx,
      });
      return items.map((item, i) => ({ ...item, current_name: newNames[i] }));
    } catch (e) {
      setNotice({ kind: "error", message: String(e) });
      return null;
    }
  };

  const onMacroStepForward = async () => {
    if (!currentMacro) return;
    if (macroStepIndex >= currentMacro.steps.length) return;
    setExecuting(true);
    try {
      const baseItems = macroItems ?? (await initMacroItems());
      if (!baseItems) return;
      const next = await applyOneStep(baseItems, currentMacro.steps[macroStepIndex]);
      if (!next) return;
      setMacroItems(next);
      setMacroStepIndex(macroStepIndex + 1);
    } finally {
      setExecuting(false);
    }
  };

  const onMacroApplyAll = async () => {
    if (!currentMacro) return;
    setExecuting(true);
    try {
      let cur = macroItems ?? (await initMacroItems());
      if (!cur) return;
      let idx = macroStepIndex;
      while (idx < currentMacro.steps.length) {
        const next = await applyOneStep(cur, currentMacro.steps[idx]);
        if (!next) return;
        cur = next;
        idx += 1;
      }
      setMacroItems(cur);
      setMacroStepIndex(idx);
    } finally {
      setExecuting(false);
    }
  };

  const onMacroReset = () => {
    setMacroItems(null);
    setMacroStepIndex(0);
  };

  const onMacroCreateNew = () => {
    const now = new Date().toISOString();
    setEditingMacro({
      id: crypto.randomUUID(),
      name: "",
      steps: [],
      created_at: now,
      updated_at: now,
    });
  };

  const onMacroEdit = (id: string) => {
    const m = macros.find((x) => x.id === id);
    if (m) setEditingMacro({ ...m, steps: m.steps.slice() });
  };

  const onMacroEditorSave = (m: Macro) => {
    setMacros((arr) => {
      const exists = arr.some((x) => x.id === m.id);
      return exists ? arr.map((x) => (x.id === m.id ? m : x)) : [...arr, m];
    });
    setCurrentMacroId(m.id);
    setEditingMacro(null);
    // 編集後に実行状態をリセット
    setMacroItems(null);
    setMacroStepIndex(0);
  };

  const onMacroEditorDelete = () => {
    if (!editingMacro) return;
    setMacros((arr) => arr.filter((x) => x.id !== editingMacro.id));
    if (currentMacroId === editingMacro.id) setCurrentMacroId(null);
    setEditingMacro(null);
    setMacroItems(null);
    setMacroStepIndex(0);
  };

  const onMacroSelectChange = (id: string | null) => {
    setCurrentMacroId(id);
    setMacroItems(null);
    setMacroStepIndex(0);
  };

  return (
    <div className={styles.app}>
      <Toolbar
        target={target}
        onTargetChange={setTarget}
        recursive={recursive}
        onRecursiveChange={setRecursive}
        depth={depth}
        onDepthChange={setDepth}
        filter={filter}
        onFilterChange={setFilter}
        filterHistory={filterHistory}
        onFilterCommit={(v) =>
          setFilterHistory((prev) => pushHistory(prev, v))
        }
      />

      <div className={styles.main}>
        <div className={styles.left}>
          <FolderTree folder={folder} onFolderChange={setFolder} />
        </div>

        <div className={styles.center}>
          <ModePanel
            mode={mode}
            onModeChange={setMode}
            advancedTab={advancedTab}
            onAdvancedTabChange={setAdvancedTab}
            regexSearch={regexSearch}
            regexReplace={regexReplace}
            onRegexSearchChange={setRegexSearch}
            onRegexReplaceChange={setRegexReplace}
            wildcardSearch={wildcardSearch}
            wildcardReplace={wildcardReplace}
            onWildcardSearchChange={setWildcardSearch}
            onWildcardReplaceChange={setWildcardReplace}
            charFrom={charFrom}
            charTo={charTo}
            onCharFromChange={setCharFrom}
            onCharToChange={setCharTo}
            builtinOp={builtinOp}
            onBuiltinKindSelect={(kind: BuiltinKind) =>
              setBuiltinOp(initialOp(kind))
            }
            onBuiltinOpChange={setBuiltinOp}
            macros={macros}
            currentMacroId={currentMacroId}
            macroStepIndex={macroStepIndex}
            macroHasItems={macroItems !== null || !!folder}
            onMacroSelect={onMacroSelectChange}
            onMacroCreateNew={onMacroCreateNew}
            onMacroEdit={onMacroEdit}
            onMacroStepForward={onMacroStepForward}
            onMacroApplyAll={onMacroApplyAll}
            onMacroReset={onMacroReset}
          />
          <SequencePanel seq={seq} onSeqChange={setSeq} />
        </div>

        <div className={styles.right}>
          <PreviewPanel
            items={displayItems}
            loading={displayLoading}
            error={displayError}
            selectedPaths={selectedPaths}
            onSelectionChange={setSelectedPaths}
          />
        </div>
      </div>

      {editingMacro && (
        <MacroEditor
          macro={editingMacro}
          onSave={onMacroEditorSave}
          onDelete={onMacroEditorDelete}
          onCancel={() => setEditingMacro(null)}
        />
      )}

      {notice && (
        <div
          className={
            notice.kind === "error" ? styles.noticeError : styles.noticeSuccess
          }
        >
          <span>{notice.message}</span>
          <button
            type="button"
            className={styles.noticeClose}
            onClick={() => setNotice(null)}
          >
            ×
          </button>
        </div>
      )}

      <ActionBar
        canRename={canRename}
        canUndo={canUndo}
        undoCount={undoStack.length}
        onRename={onRename}
        onUndo={onUndo}
        onClear={() => {
          setSelectedPaths(new Set());
          preview.reload();
        }}
      />
    </div>
  );
}
