import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import styles from "./FolderTree.module.css";

export interface FolderTreeProps {
  folder: string;
  onFolderChange: (path: string) => void;
}

interface DirNode {
  name: string;
  path: string;
  has_children: boolean;
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

interface TreeNodeProps {
  node: DirNode;
  selectedPath: string;
  expanded: Set<string>;
  childrenMap: Map<string, DirNode[]>;
  loading: Set<string>;
  onToggle: (node: DirNode) => void;
  onSelect: (path: string) => void;
}

function TreeNode({
  node,
  selectedPath,
  expanded,
  childrenMap,
  loading,
  onToggle,
  onSelect,
}: TreeNodeProps) {
  const isOpen = expanded.has(node.path);
  const isLoading = loading.has(node.path);
  const isSelected = normalizeForCompare(selectedPath) === normalizeForCompare(node.path);
  const kids = childrenMap.get(node.path);

  return (
    <div className={styles.subtree}>
      <div
        className={`${styles.row} ${isSelected ? styles.rowSelected : ""}`}
        onClick={() => onSelect(node.path)}
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
        <span className={styles.name}>{node.name}</span>
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
              onToggle={onToggle}
              onSelect={onSelect}
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

export default function FolderTree({ folder, onFolderChange }: FolderTreeProps) {
  const [roots, setRoots] = useState<DirNode[]>([]);
  const [childrenMap, setChildrenMap] = useState<Map<string, DirNode[]>>(new Map());
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState<Set<string>>(new Set());
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
        {roots.map((r) => (
          <TreeNode
            key={r.path}
            node={r}
            selectedPath={folder}
            expanded={expanded}
            childrenMap={childrenMap}
            loading={loading}
            onToggle={onToggle}
            onSelect={onSelect}
          />
        ))}
      </div>
      <div className={styles.path} title={folder}>
        {folder || <span className={styles.placeholder}>未選択</span>}
      </div>
    </div>
  );
}
