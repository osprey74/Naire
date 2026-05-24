import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, ask } from "@tauri-apps/plugin-dialog";
import type { RenameRecord } from "../types/rename";
import styles from "./FolderTree.module.css";

export interface FolderTreeProps {
  folder: string;
  onFolderChange: (path: string) => void;
  onFolderRenamed?: (record: RenameRecord, oldPath: string, newPath: string) => void;
  onFolderDeleted?: (path: string) => void;
  onError?: (message: string) => void;
  /// 親コンポーネントからのリロード要求シグナル。値が変化するたびに
  /// ルート再取得 + 展開中フォルダの children 再取得 + スタッツキャッシュ破棄を行う。
  reloadSignal?: number;
}

interface DirNode {
  name: string;
  path: string;
  has_children: boolean;
}

interface FolderStats {
  file_count: number;
  total_size: number;
}

const SEP_RE = /[\\/]+/;

function parentOf(path: string): string | null {
  if (!path) return null;
  const norm = path.replace(/\\/g, "/").replace(/\/+$/, "");
  const idx = norm.lastIndexOf("/");
  if (idx <= 0) return null;
  const parent = norm.slice(0, idx);
  if (/^[A-Za-z]:$/.test(parent)) return parent + "/";
  return parent;
}

function normalizeForCompare(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
}

/// `target` が `prefix` の配下にある場合、prefix から target までのフォルダ列を
/// `prefix` で始まる絶対パス列で返す（prefix 自身は含まない）。
function buildDescendantChain(prefix: string, target: string): string[] {
  const pNorm = normalizeForCompare(prefix);
  const tNorm = normalizeForCompare(target);
  if (!tNorm.startsWith(pNorm)) return [];
  const tail = target
    .replace(/\\/g, "/")
    .slice(prefix.replace(/\\/g, "/").length)
    .split(SEP_RE)
    .filter(Boolean);
  if (tail.length === 0) return [];
  const usesBackslash = prefix.includes("\\") || target.includes("\\");
  const sep = usesBackslash ? "\\" : "/";
  const chain: string[] = [];
  let cur = prefix.replace(/[\\/]+$/, "");
  for (const part of tail) {
    cur = cur + sep + part;
    chain.push(cur);
  }
  return chain;
}

/// パス `p` が `prefix` 配下（または同一）かを大小無視・区切り正規化で判定。
function isPathUnder(p: string, prefix: string): boolean {
  const a = normalizeForCompare(p);
  const b = normalizeForCompare(prefix);
  if (a === b) return true;
  return a.startsWith(b + "/");
}

/// `p` が `oldPrefix` 配下（または同一）のとき、prefix を `newPrefix` に置換して返す。
/// それ以外は null。区切り文字は元の `p` のものを尊重する。
function rewritePath(p: string, oldPrefix: string, newPrefix: string): string | null {
  if (!isPathUnder(p, oldPrefix)) return null;
  const oldNorm = oldPrefix.replace(/\\/g, "/").replace(/\/+$/, "");
  const pSlash = p.replace(/\\/g, "/");
  const tail = pSlash.slice(oldNorm.length); // 先頭が "/" or 空
  const usesBackslash = p.includes("\\");
  const newPrefixTrim = newPrefix.replace(/\\/g, "/").replace(/\/+$/, "");
  const merged = newPrefixTrim + tail;
  return usesBackslash ? merged.replace(/\//g, "\\") : merged;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(1)} MB`;
  const gb = mb / 1024;
  return `${gb.toFixed(2)} GB`;
}

/// ドライブルート（Windows の `C:\` 等）の判定。重い再帰集計を避けるため
/// 自動取得対象から除外する。
function isDriveRoot(path: string): boolean {
  return /^[A-Za-z]:[\\/]?$/.test(path);
}

interface TreeNodeProps {
  node: DirNode;
  selectedPath: string;
  expanded: Set<string>;
  childrenMap: Map<string, DirNode[]>;
  loading: Set<string>;
  statsMap: Map<string, FolderStats | "loading" | "error">;
  editingPath: string | null;
  onRequestStats: (path: string) => void;
  onToggle: (node: DirNode) => void;
  onSelect: (path: string) => void;
  onStartRename: (path: string) => void;
  onCommitRename: (path: string, newName: string) => Promise<void>;
  onCancelRename: () => void;
  onContextMenu: (e: React.MouseEvent, node: DirNode) => void;
}

function TreeNode({
  node,
  selectedPath,
  expanded,
  childrenMap,
  loading,
  statsMap,
  editingPath,
  onRequestStats,
  onToggle,
  onSelect,
  onStartRename,
  onCommitRename,
  onCancelRename,
  onContextMenu,
}: TreeNodeProps) {
  const isOpen = expanded.has(node.path);
  const isLoading = loading.has(node.path);
  const isSelected = normalizeForCompare(selectedPath) === normalizeForCompare(node.path);
  const kids = childrenMap.get(node.path);
  const isEditing = editingPath === node.path;
  const stats = statsMap.get(node.path);
  const skipStats = isDriveRoot(node.path);

  // 初回レンダリング時にスタッツ未取得なら取得をリクエスト（ドライブルートはスキップ）
  useEffect(() => {
    if (skipStats) return;
    if (stats !== undefined) return;
    onRequestStats(node.path);
  }, [skipStats, stats, node.path, onRequestStats]);

  // 編集モード用の input ref
  const inputRef = useRef<HTMLInputElement | null>(null);
  useEffect(() => {
    if (isEditing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [isEditing]);

  const statsLabel = (() => {
    if (skipStats) return "";
    if (stats === undefined || stats === "loading") return "…";
    if (stats === "error") return "—";
    return `${stats.file_count.toLocaleString()} / ${formatSize(stats.total_size)}`;
  })();

  return (
    <div className={styles.subtree}>
      <div
        className={`${styles.row} ${isSelected ? styles.rowSelected : ""}`}
        onClick={() => {
          if (isEditing) return;
          onSelect(node.path);
        }}
        onContextMenu={(e) => {
          if (isEditing) return;
          onContextMenu(e, node);
        }}
        title={node.path}
      >
        {node.has_children ? (
          <button
            type="button"
            className={styles.arrow}
            onClick={(e) => {
              e.stopPropagation();
              onToggle(node);
            }}
            aria-label={isOpen ? "閉じる" : "開く"}
          >
            {isOpen ? "▼" : "▶"}
          </button>
        ) : (
          <span className={styles.arrowSpacer} />
        )}
        {isEditing ? (
          <input
            ref={inputRef}
            className={styles.editInput}
            defaultValue={node.name}
            aria-label="フォルダ名"
            title="フォルダ名を編集"
            onClick={(e) => e.stopPropagation()}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                onCommitRename(node.path, (e.target as HTMLInputElement).value);
              } else if (e.key === "Escape") {
                e.preventDefault();
                onCancelRename();
              }
            }}
            onBlur={(e) => {
              onCommitRename(node.path, e.target.value);
            }}
          />
        ) : (
          <span
            className={styles.name}
            onDoubleClick={(e) => {
              e.stopPropagation();
              onStartRename(node.path);
            }}
          >
            {node.name}
          </span>
        )}
        {!isEditing && (
          <span className={styles.stats} title="ファイル数 / 合計サイズ（再帰）">
            {statsLabel}
          </span>
        )}
      </div>
      {isOpen && kids && kids.length > 0 && (
        <div className={styles.children}>
          {kids.map((child) => (
            <TreeNode
              key={child.path}
              node={child}
              selectedPath={selectedPath}
              expanded={expanded}
              childrenMap={childrenMap}
              loading={loading}
              statsMap={statsMap}
              editingPath={editingPath}
              onRequestStats={onRequestStats}
              onToggle={onToggle}
              onSelect={onSelect}
              onStartRename={onStartRename}
              onCommitRename={onCommitRename}
              onCancelRename={onCancelRename}
              onContextMenu={onContextMenu}
            />
          ))}
        </div>
      )}
      {isOpen && !kids && isLoading && (
        <div className={styles.children}>
          <div className={styles.loading}>読み込み中…</div>
        </div>
      )}
    </div>
  );
}

interface ContextMenuState {
  path: string;
  name: string;
  x: number;
  y: number;
}

export default function FolderTree({
  folder,
  onFolderChange,
  onFolderRenamed,
  onFolderDeleted,
  onError,
  reloadSignal,
}: FolderTreeProps) {
  const [roots, setRoots] = useState<DirNode[]>([]);
  const [childrenMap, setChildrenMap] = useState<Map<string, DirNode[]>>(new Map());
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState<Set<string>>(new Set());
  const [statsMap, setStatsMap] = useState<Map<string, FolderStats | "loading" | "error">>(
    new Map(),
  );
  const [editingPath, setEditingPath] = useState<string | null>(null);
  const [renameError, setRenameError] = useState<string | null>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  // onBlur と Enter/Escape の二重発火を抑止するためのフラグ
  const renameInFlightRef = useRef<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const autoExpandedRef = useRef(false);

  const fetchChildren = useCallback(
    async (path: string): Promise<DirNode[] | null> => {
      setLoading((s) => {
        const next = new Set(s);
        next.add(path);
        return next;
      });
      try {
        const list = await invoke<DirNode[]>("list_folder_tree", { path });
        setChildrenMap((m) => {
          const next = new Map(m);
          next.set(path, list);
          return next;
        });
        return list;
      } catch (e) {
        setError(String(e));
        return null;
      } finally {
        setLoading((s) => {
          const next = new Set(s);
          next.delete(path);
          return next;
        });
      }
    },
    [],
  );

  const requestStats = useCallback((path: string) => {
    setStatsMap((m) => {
      if (m.has(path)) return m;
      const next = new Map(m);
      next.set(path, "loading");
      return next;
    });
    (async () => {
      try {
        const s = await invoke<FolderStats>("get_folder_stats", { path });
        setStatsMap((m) => {
          const next = new Map(m);
          next.set(path, s);
          return next;
        });
      } catch {
        setStatsMap((m) => {
          const next = new Map(m);
          next.set(path, "error");
          return next;
        });
      }
    })();
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const list = await invoke<DirNode[]>("list_folder_tree", { path: null });
        if (!cancelled) setRoots(list);
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  // 親からのリロード要求シグナル。初回マウントの取りこぼし防止のため
  // undefined / 初期値時はスキップし、値の変化のみで再取得する。
  // expanded は最新値を ref 経由で読み、deps には含めない（展開操作で誤発火させない）。
  const initialReloadRef = useRef(reloadSignal);
  const expandedRef = useRef(expanded);
  expandedRef.current = expanded;
  useEffect(() => {
    if (reloadSignal === undefined) return;
    if (reloadSignal === initialReloadRef.current) return;
    let cancelled = false;
    (async () => {
      try {
        const list = await invoke<DirNode[]>("list_folder_tree", { path: null });
        if (!cancelled) setRoots(list);
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
      if (cancelled) return;
      // スタッツキャッシュを破棄して再計算させる
      setStatsMap(new Map());
      // 現在展開中のフォルダの children を一括再取得
      const targets = Array.from(expandedRef.current);
      for (const p of targets) {
        if (cancelled) return;
        await fetchChildren(p);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [reloadSignal, fetchChildren]);

  // 起動時に保存されていた folder までツリーを展開
  useEffect(() => {
    if (autoExpandedRef.current) return;
    if (!folder || roots.length === 0) return;
    autoExpandedRef.current = true;
    const root = roots.find((r) =>
      normalizeForCompare(folder).startsWith(normalizeForCompare(r.path)),
    );
    if (!root) return;
    (async () => {
      const chain = buildDescendantChain(root.path, folder);
      const toExpand = [root.path, ...chain.slice(0, -1)];
      setExpanded((s) => {
        const next = new Set(s);
        toExpand.forEach((p) => next.add(p));
        return next;
      });
      for (const p of toExpand) {
        await fetchChildren(p);
      }
    })();
  }, [folder, roots, fetchChildren]);

  const onToggle = (node: DirNode) => {
    const isOpen = expanded.has(node.path);
    setExpanded((s) => {
      const next = new Set(s);
      if (isOpen) next.delete(node.path);
      else next.add(node.path);
      return next;
    });
    if (!isOpen && !childrenMap.has(node.path)) {
      fetchChildren(node.path);
    }
  };

  const onSelect = (path: string) => {
    onFolderChange(path);
  };

  const onStartRename = (path: string) => {
    setRenameError(null);
    setEditingPath(path);
  };

  const onContextMenu = useCallback((e: React.MouseEvent, node: DirNode) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ path: node.path, name: node.name, x: e.clientX, y: e.clientY });
  }, []);

  // メニュー外クリック / Esc で閉じる
  useEffect(() => {
    if (!contextMenu) return;
    const onDocClick = () => setContextMenu(null);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setContextMenu(null);
    };
    // capture フェーズで listen → メニュー項目クリック後に setContextMenu(null) する流れと両立する
    document.addEventListener("click", onDocClick);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("click", onDocClick);
      document.removeEventListener("keydown", onKey);
    };
  }, [contextMenu]);

  const onDeleteFolder = useCallback(
    async (path: string, name: string) => {
      setContextMenu(null);
      const confirmed = await ask(
        `「${name}」をゴミ箱へ移動しますか？\n\n${path}`,
        { title: "フォルダを削除", kind: "warning" },
      );
      if (!confirmed) return;
      try {
        await invoke<void>("move_folder_to_trash", { path });

        // 親フォルダの children を再取得
        const parent = parentOf(path);
        if (parent !== null) {
          await fetchChildren(parent);
        } else {
          try {
            const list = await invoke<DirNode[]>("list_folder_tree", { path: null });
            setRoots(list);
          } catch {
            // ignore
          }
        }

        // childrenMap / expanded / statsMap から削除パス配下のエントリを掃除
        setChildrenMap((m) => {
          const next = new Map<string, DirNode[]>();
          for (const [k, v] of m) {
            if (isPathUnder(k, path)) continue;
            next.set(k, v);
          }
          return next;
        });
        setExpanded((s) => {
          const next = new Set<string>();
          for (const p of s) {
            if (isPathUnder(p, path)) continue;
            next.add(p);
          }
          return next;
        });
        setStatsMap((m) => {
          const next = new Map<string, FolderStats | "loading" | "error">();
          for (const [k, v] of m) {
            if (isPathUnder(k, path)) continue;
            next.set(k, v);
          }
          return next;
        });

        // 現在選択中フォルダが削除対象配下なら、親へフォールバック
        if (folder && isPathUnder(folder, path)) {
          const p = parentOf(path);
          onFolderChange(p ?? "");
        }

        onFolderDeleted?.(path);
      } catch (e) {
        onError?.(String(e));
      }
    },
    [fetchChildren, folder, onFolderChange, onFolderDeleted, onError],
  );

  const onCancelRename = () => {
    setEditingPath(null);
    setRenameError(null);
  };

  const onCommitRename = useCallback(
    async (oldPath: string, newNameRaw: string) => {
      // 二重発火防止（onBlur と Enter の競合）
      if (renameInFlightRef.current.has(oldPath)) return;
      const newName = newNameRaw.trim();
      const currentName =
        oldPath.replace(/[\\/]+$/, "").split(SEP_RE).pop() ?? "";
      if (newName === "" || newName === currentName) {
        // 変更なし or 空 → キャンセル扱い
        setEditingPath(null);
        setRenameError(null);
        return;
      }
      renameInFlightRef.current.add(oldPath);
      try {
        const record = await invoke<RenameRecord>("rename_folder", {
          oldPath,
          newName,
        });
        // 新パスを記録から取り出す
        const newPath =
          record.ops[0]?.type === "rename"
            ? record.ops[0].new_path
            : oldPath;

        // 親フォルダの children を再取得（フラグメントを最新化）
        const parent = parentOf(oldPath);
        if (parent !== null) {
          await fetchChildren(parent);
        } else {
          // ルート扱い: roots を再取得
          try {
            const list = await invoke<DirNode[]>("list_folder_tree", { path: null });
            setRoots(list);
          } catch {
            // ignore
          }
        }

        // childrenMap / expanded / statsMap から旧パス配下のエントリを掃除
        setChildrenMap((m) => {
          const next = new Map<string, DirNode[]>();
          for (const [k, v] of m) {
            if (isPathUnder(k, oldPath)) continue; // 旧パス配下キーは破棄
            next.set(k, v);
          }
          return next;
        });
        setExpanded((s) => {
          const next = new Set<string>();
          for (const p of s) {
            if (isPathUnder(p, oldPath)) {
              const remapped = rewritePath(p, oldPath, newPath);
              if (remapped) next.add(remapped);
            } else {
              next.add(p);
            }
          }
          return next;
        });
        setStatsMap((m) => {
          const next = new Map<string, FolderStats | "loading" | "error">();
          for (const [k, v] of m) {
            if (isPathUnder(k, oldPath)) continue;
            next.set(k, v);
          }
          return next;
        });

        // 現在選択中フォルダが旧パス配下なら新パスに付け替え
        if (folder && isPathUnder(folder, oldPath)) {
          const remapped = rewritePath(folder, oldPath, newPath);
          if (remapped) onFolderChange(remapped);
        }

        onFolderRenamed?.(record, oldPath, newPath);
        setEditingPath(null);
        setRenameError(null);
      } catch (e) {
        setRenameError(String(e));
        // エラー時は編集モードを維持してユーザに再入力させる
      } finally {
        renameInFlightRef.current.delete(oldPath);
      }
    },
    [fetchChildren, folder, onFolderChange, onFolderRenamed],
  );

  const pickDialog = async () => {
    const picked = await open({
      directory: true,
      multiple: false,
      defaultPath: folder || undefined,
    });
    if (typeof picked === "string") {
      onFolderChange(picked);
    }
  };

  const goUp = () => {
    const parent = parentOf(folder);
    if (parent) onFolderChange(parent);
  };

  return (
    <div className={styles.tree}>
      <div className={styles.actions}>
        <button type="button" onClick={pickDialog} className={styles.button}>
          場所を選択…
        </button>
        <button
          type="button"
          onClick={goUp}
          disabled={!folder || parentOf(folder) === null}
          className={styles.button}
          title="親フォルダへ"
        >
          ↑
        </button>
      </div>
      <div className={styles.nodes}>
        {error && <div className={styles.error}>{error}</div>}
        {renameError && (
          <div className={styles.error}>リネーム失敗: {renameError}</div>
        )}
        {roots.map((r) => (
          <TreeNode
            key={r.path}
            node={r}
            selectedPath={folder}
            expanded={expanded}
            childrenMap={childrenMap}
            loading={loading}
            statsMap={statsMap}
            editingPath={editingPath}
            onRequestStats={requestStats}
            onToggle={onToggle}
            onSelect={onSelect}
            onStartRename={onStartRename}
            onCommitRename={onCommitRename}
            onCancelRename={onCancelRename}
            onContextMenu={onContextMenu}
          />
        ))}
      </div>
      <div className={styles.path} title={folder}>
        {folder || <span className={styles.placeholder}>未選択</span>}
      </div>
      {contextMenu && (
        <div
          className={styles.contextMenu}
          style={{ left: contextMenu.x, top: contextMenu.y }}
          onClick={(e) => e.stopPropagation()}
        >
          <button
            type="button"
            className={styles.contextMenuItem}
            onClick={() => onDeleteFolder(contextMenu.path, contextMenu.name)}
          >
            フォルダを削除（ゴミ箱へ移動）
          </button>
        </div>
      )}
    </div>
  );
}
