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
import AboutDialog from "./components/AboutDialog";
import type { GroupState } from "./components/ModePanel/group/GroupPanel";
import { usePreview } from "./hooks/usePreview";
import { persistConfig, useLoadConfig } from "./hooks/useConfig";
import { pushHistory } from "./components/FilterCombo";
import {
  initialOp,
  type BuiltinKind,
} from "./components/ModePanel/builtin/builtin-defaults";
import type {
  AppConfig,
  BuiltinOp,
  GroupPreview,
  GroupRenameDto,
  Macro,
  PreviewColumnWidths,
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

const DEFAULT_COLUMN_WIDTHS: PreviewColumnWidths = {
  original: 240,
  renamed: 240,
};

const DEFAULT_LEFT_WIDTH = 220;
const MIN_LEFT_WIDTH = 140;
const MAX_LEFT_WIDTH = 800;

const DEFAULT_GROUP_STATE: GroupState = {
  groupName: "",
  nameDirty: false,
  renameEnabled: true,
  prefix: "",
  suffix: "",
  digits: 2,
  start: 1,
  step: 1,
};

type Notice = { kind: "success" | "error"; message: string };

// パスから親ディレクトリ部分を取り出す（Windows / POSIX 両対応）。
// 末尾セパレータの直前で切る。セパレータが見つからなければ空文字。
function dirname(p: string): string {
  const idx = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
  if (idx < 0) return "";
  return p.substring(0, idx);
}

export default function App() {
  const { loaded: configLoaded, initialConfig } = useLoadConfig();
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
  const [columnWidths, setColumnWidths] = useState<PreviewColumnWidths>(
    DEFAULT_COLUMN_WIDTHS,
  );
  const [leftWidth, setLeftWidth] = useState<number>(DEFAULT_LEFT_WIDTH);
  const [resizing, setResizing] = useState(false);
  // フォルダツリーへのリロード要求カウンタ。インクリメントで FolderTree が再取得する。
  const [folderTreeReload, setFolderTreeReload] = useState(0);
  // ── マクロ関連の state ──────────────────────────────────────
  const [macros, setMacros] = useState<Macro[]>([]);
  const [currentMacroId, setCurrentMacroId] = useState<string | null>(null);
  const [editingMacro, setEditingMacro] = useState<Macro | null>(null);
  const [macroItems, setMacroItems] = useState<StepItem[] | null>(null);
  const [macroStepIndex, setMacroStepIndex] = useState(0);
  // ── フォルダ集約関連の state（Phase 14）─────────────────────
  const [groupState, setGroupState] = useState<GroupState>(DEFAULT_GROUP_STATE);
  const [groupPreview, setGroupPreview] = useState<GroupPreview | null>(null);
  const [groupComputing, setGroupComputing] = useState(false);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [executing, setExecuting] = useState(false);
  const [aboutOpen, setAboutOpen] = useState(false);
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
  const isGroupMode = mode === "group";
  const currentMacro = isMacroMode
    ? macros.find((m) => m.id === currentMacroId) ?? null
    : null;

  // ── フォルダ集約：選択から親フォルダ・名前を導出 ────────────
  // 選択された PreviewItem を抽出 → 同一親に属しているか検証 → 親と names を返す。
  const groupSelection = useMemo(() => {
    const selectedItems = lastItems.filter((i) => selectedPaths.has(i.path));
    if (selectedItems.length === 0) {
      return { parent: "", names: [] as string[], sameParent: true };
    }
    const parents = new Set(selectedItems.map((i) => dirname(i.path)));
    const sameParent = parents.size === 1;
    return {
      parent: sameParent ? dirname(selectedItems[0].path) : "",
      names: selectedItems.map((i) => i.original),
      sameParent,
    };
  }, [lastItems, selectedPaths]);

  // 集約モード時に groupPreview を計算（state 変更で debounce 再計算）
  const groupRenameDto = useMemo<GroupRenameDto | null>(() => {
    if (!groupState.renameEnabled) return null;
    return {
      prefix: groupState.prefix,
      suffix: groupState.suffix,
      digits: groupState.digits,
      start: groupState.start,
      step: groupState.step,
      numbering: seq.numbering,
    };
  }, [groupState, seq.numbering]);

  useEffect(() => {
    if (!isGroupMode) {
      setGroupPreview(null);
      return;
    }
    if (target !== "folder") {
      setGroupPreview(null);
      return;
    }
    if (groupSelection.names.length < 2 || !groupSelection.sameParent) {
      setGroupPreview(null);
      return;
    }
    const t = setTimeout(async () => {
      setGroupComputing(true);
      try {
        const preview = await invoke<GroupPreview>("compute_group_preview", {
          parentFolder: groupSelection.parent,
          selectedNames: groupSelection.names,
          groupName: groupState.nameDirty ? groupState.groupName : null,
          rename: groupRenameDto,
        });
        setGroupPreview(preview);
        // 自動算出時は groupState.groupName を共通プレフィックスに同期
        if (!groupState.nameDirty && preview.common_prefix !== groupState.groupName) {
          setGroupState((prev) =>
            prev.nameDirty
              ? prev
              : { ...prev, groupName: preview.common_prefix },
          );
        }
      } catch (e) {
        setGroupPreview({
          parent_folder: groupSelection.parent,
          common_prefix: "",
          group_name: groupState.groupName,
          conflict: false,
          items: [],
          error: String(e),
        });
      } finally {
        setGroupComputing(false);
      }
    }, 200);
    return () => clearTimeout(t);
  }, [
    isGroupMode,
    target,
    groupSelection.parent,
    groupSelection.names.join(""),
    groupSelection.sameParent,
    groupState.groupName,
    groupState.nameDirty,
    groupRenameDto,
  ]);

  // 集約モードのフォルダ非選択時 / 異親選択時 / 件数不足時のメッセージ表示用
  const groupPreviewError = useMemo<string | null>(() => {
    if (!isGroupMode) return null;
    if (target !== "folder") return null; // GroupPanel 側で案内
    if (groupSelection.names.length === 0) return null; // 同上
    if (groupSelection.names.length < 2) return null; // 同上
    if (!groupSelection.sameParent) {
      return "選択フォルダが複数の親に分散しています。同じ親フォルダの下から選択してください。";
    }
    return null;
  }, [isGroupMode, target, groupSelection.names.length, groupSelection.sameParent]);

  // GroupPanel 表示用の合成 preview（異親エラーは preview に上書き反映）
  const effectiveGroupPreview = useMemo<GroupPreview | null>(() => {
    if (!isGroupMode) return null;
    if (groupPreviewError) {
      return {
        parent_folder: "",
        common_prefix: "",
        group_name: groupState.groupName,
        conflict: false,
        items: [],
        error: groupPreviewError,
      };
    }
    return groupPreview;
  }, [isGroupMode, groupPreviewError, groupPreview, groupState.groupName]);

  // 表示する items: マクロモード中は macroAsPreview、それ以外は通常 preview
  // 集約モードでは PreviewPanel は通常のフォルダ一覧を表示し続ける
  // （items を切り替えると PreviewPanel が selectedPaths を path 不一致で間引いてしまうため）。
  // 集約プレビューは GroupPanel 内の表で表示する。
  const displayItems = isMacroMode && macroItems
    ? macroAsPreview
    : preview.items;
  const displayLoading = isMacroMode
    ? executing
    : isGroupMode
      ? preview.loading || groupComputing || executing
      : preview.loading;
  const displayError = isMacroMode ? null : preview.error;

  const hasChanges = displayItems.some((i) => i.is_changed);
  // 集約モードは独自の「集約実行」ボタンを持つため、ActionBar の「リネーム実行」は無効化
  const canRename =
    !isGroupMode && !!folder && hasChanges && !displayLoading && !executing;
  const canUndo = undoStack.length > 0 && !executing;

  // 集約モード: 「集約実行」可能か
  const canGroupExecute = !!(
    isGroupMode &&
    target === "folder" &&
    groupSelection.sameParent &&
    groupSelection.names.length >= 2 &&
    groupPreview &&
    !groupPreview.error &&
    !groupPreview.conflict &&
    !groupComputing &&
    !executing
  );

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

  const onGroupExecute = async () => {
    if (!canGroupExecute || !groupPreview) return;
    setExecuting(true);
    try {
      const record = await invoke<RenameRecord>("execute_group", {
        parentFolder: groupSelection.parent,
        selectedNames: groupSelection.names,
        groupName: groupPreview.group_name,
        rename: groupRenameDto,
      });
      setUndoStack((stack) => [...stack, record].slice(-20));
      setNotice({
        kind: "success",
        message: `${groupSelection.names.length} 件のフォルダを「${groupPreview.group_name}」に集約しました`,
      });
      // 集約状態をリセット
      setSelectedPaths(new Set());
      setGroupState(DEFAULT_GROUP_STATE);
      setGroupPreview(null);
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

  // 左ペイン幅のドラッグリサイズ。pointermove/up を window に張る。
  const onResizerPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    e.preventDefault();
    setResizing(true);
    const startX = e.clientX;
    const startWidth = leftWidth;
    const onMove = (ev: PointerEvent) => {
      const next = Math.max(
        MIN_LEFT_WIDTH,
        Math.min(MAX_LEFT_WIDTH, startWidth + (ev.clientX - startX)),
      );
      setLeftWidth(next);
    };
    const onUp = () => {
      setResizing(false);
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
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

  // ── 設定の永続化 ──────────────────────────────────────────
  // 起動時に 1 回だけ initialConfig を state へ反映する。
  // 適用が完了するまで debounce 保存はスキップ（ロードした値を上書きで保存しない）。
  const [configApplied, setConfigApplied] = useState(false);
  useEffect(() => {
    if (!configLoaded) return;
    if (initialConfig) {
      if (initialConfig.last_folder) setFolder(initialConfig.last_folder);
      if (initialConfig.last_mode) setMode(initialConfig.last_mode);
      if (initialConfig.last_target) setTarget(initialConfig.last_target);
      if (typeof initialConfig.recursive === "boolean")
        setRecursive(initialConfig.recursive);
      if (typeof initialConfig.depth === "number") setDepth(initialConfig.depth);
      if (initialConfig.filter) setFilter(initialConfig.filter);
      if (Array.isArray(initialConfig.filter_history))
        setFilterHistory(initialConfig.filter_history);
      if (initialConfig.seq) setSeq(initialConfig.seq);
      if (Array.isArray(initialConfig.macros)) setMacros(initialConfig.macros);
      if (
        initialConfig.column_widths &&
        typeof initialConfig.column_widths.original === "number" &&
        typeof initialConfig.column_widths.renamed === "number"
      ) {
        setColumnWidths(initialConfig.column_widths);
      }
      if (typeof initialConfig.left_width === "number") {
        const w = Math.max(
          MIN_LEFT_WIDTH,
          Math.min(MAX_LEFT_WIDTH, initialConfig.left_width),
        );
        setLeftWidth(w);
      }
    }
    setConfigApplied(true);
  }, [configLoaded, initialConfig]);

  // 関連 state が変わるたびに debounced で保存。configApplied が true になるまでは
  // 走らせない（起動直後にデフォルト値を上書き保存しないため）。
  useEffect(() => {
    if (!configApplied) return;
    const cfg: AppConfig = {
      last_folder: folder,
      last_mode: mode,
      last_target: target,
      recursive,
      depth,
      filter,
      filter_history: filterHistory,
      seq,
      macros,
      column_widths: columnWidths,
      left_width: leftWidth,
    };
    const t = setTimeout(() => persistConfig(cfg), 500);
    return () => clearTimeout(t);
  }, [
    configApplied,
    folder,
    mode,
    target,
    recursive,
    depth,
    filter,
    filterHistory,
    seq,
    macros,
    columnWidths,
    leftWidth,
  ]);

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

  // 集約モード以外に切り替えたら集約状態をリセット
  useEffect(() => {
    if (mode !== "group") {
      setGroupState(DEFAULT_GROUP_STATE);
      setGroupPreview(null);
    }
  }, [mode]);

  // フォルダ / target / 再帰条件 / フィルタが変わったら集約状態もリセット
  useEffect(() => {
    setGroupState(DEFAULT_GROUP_STATE);
    setGroupPreview(null);
  }, [folder, target, recursive, depth, filter]);

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

  const onMacroImport = async () => {
    try {
      const imported = await invoke<Macro[]>("import_macros");
      if (imported.length === 0) {
        setNotice({ kind: "error", message: "マクロが読み込まれませんでした" });
        return;
      }
      setMacros((arr) => [...arr, ...imported]);
      setNotice({
        kind: "success",
        message: `${imported.length} 件のマクロをインポートしました`,
      });
    } catch (e) {
      const msg = String(e);
      // ユーザがキャンセルしたときはエラー通知しない
      if (!msg.includes("キャンセル")) {
        setNotice({ kind: "error", message: msg });
      }
    }
  };

  const onMacroExportAll = async () => {
    if (macros.length === 0) return;
    try {
      await invoke<void>("export_macros", { macros });
      setNotice({
        kind: "success",
        message: `${macros.length} 件のマクロをエクスポートしました`,
      });
    } catch (e) {
      const msg = String(e);
      if (!msg.includes("キャンセル")) {
        setNotice({ kind: "error", message: msg });
      }
    }
  };

  const onMacroEditorExport = async (m: Macro) => {
    try {
      await invoke<void>("export_macros", { macros: [m] });
      setNotice({
        kind: "success",
        message: `「${m.name || "(無題)"}」をエクスポートしました`,
      });
    } catch (e) {
      const msg = String(e);
      if (!msg.includes("キャンセル")) {
        setNotice({ kind: "error", message: msg });
      }
    }
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
        onAboutClick={() => setAboutOpen(true)}
      />

      <div
        className={styles.main}
        style={{ ["--left-width" as string]: `${leftWidth}px` }}
      >
        <div className={styles.left}>
          <FolderTree
            folder={folder}
            onFolderChange={setFolder}
            reloadSignal={folderTreeReload}
            onFolderRenamed={(record) => {
              setUndoStack((stack) => [...stack, record].slice(-20));
              setNotice({
                kind: "success",
                message: "フォルダ名を変更しました",
              });
              preview.reload();
            }}
            onFolderDeleted={(path) => {
              setNotice({
                kind: "success",
                message: `「${path}」をゴミ箱へ移動しました`,
              });
              preview.reload();
            }}
            onError={(msg) => setNotice({ kind: "error", message: msg })}
          />
        </div>
        <div
          className={`${styles.resizer} ${resizing ? styles.resizerActive : ""}`}
          onPointerDown={onResizerPointerDown}
          role="separator"
          aria-orientation="vertical"
          aria-label="フォルダツリーの幅"
          title="ドラッグして幅を調整"
        />

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
            onMacroImport={onMacroImport}
            onMacroExportAll={onMacroExportAll}
            onMacroStepForward={onMacroStepForward}
            onMacroApplyAll={onMacroApplyAll}
            onMacroReset={onMacroReset}
            target={target}
            groupSelectedCount={groupSelection.names.length}
            groupState={groupState}
            onGroupStateChange={setGroupState}
            groupPreview={effectiveGroupPreview}
            groupComputing={groupComputing}
            onGroupExecute={onGroupExecute}
            canGroupExecute={canGroupExecute}
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
            columnWidths={columnWidths}
            onColumnWidthsChange={setColumnWidths}
          />
        </div>
      </div>

      {editingMacro && (
        <MacroEditor
          macro={editingMacro}
          onSave={onMacroEditorSave}
          onDelete={onMacroEditorDelete}
          onCancel={() => setEditingMacro(null)}
          onExport={onMacroEditorExport}
        />
      )}

      {aboutOpen && <AboutDialog onClose={() => setAboutOpen(false)} />}

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
          preview.reload();
          setFolderTreeReload((n) => n + 1);
        }}
      />
    </div>
  );
}
