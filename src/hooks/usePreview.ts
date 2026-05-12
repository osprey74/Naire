import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  PreviewItem,
  RenameStep,
  SequenceConfig,
  TargetType,
} from "../types/rename";

export interface PreviewArgs {
  folder: string;
  steps: RenameStep[];
  target: TargetType;
  recursive: boolean;
  depth: number;
  filter: string;
  seq: SequenceConfig;
  selectedIndexes: number[];
}

export interface PreviewState {
  items: PreviewItem[];
  error: string | null;
  loading: boolean;
  reload: () => void;
}

export function usePreview(args: PreviewArgs): PreviewState {
  const [items, setItems] = useState<PreviewItem[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [reloadKey, setReloadKey] = useState(0);

  const depKey = JSON.stringify(args);

  useEffect(() => {
    if (!args.folder) {
      setItems([]);
      setError(null);
      setLoading(false);
      return;
    }

    let cancelled = false;
    setLoading(true);
    setError(null);

    invoke<PreviewItem[]>("preview_rename", {
      folder: args.folder,
      steps: args.steps,
      target: args.target,
      recursive: args.recursive,
      depth: args.depth,
      filter: args.filter,
      seq: args.seq,
      selectedIndexes: args.selectedIndexes,
    })
      .then((result) => {
        if (cancelled) return;
        setItems(result);
        setLoading(false);
      })
      .catch((e) => {
        if (cancelled) return;
        setError(String(e));
        setItems([]);
        setLoading(false);
      });

    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [depKey, reloadKey]);

  return {
    items,
    error,
    loading,
    reload: () => setReloadKey((k) => k + 1),
  };
}
