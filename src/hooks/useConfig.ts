import { useEffect, useState } from "react";
import { load, type Store } from "@tauri-apps/plugin-store";
import type { AppConfig } from "../types/rename";

const STORE_FILE = "config.json";
const KEY = "config";

// Store インスタンスはアプリ全体で 1 つに固定する（複数 load() を避ける）。
let storePromise: Promise<Store> | null = null;
function getStore(): Promise<Store> {
  if (!storePromise) {
    storePromise = load(STORE_FILE);
  }
  return storePromise;
}

export interface UseConfigResult {
  /// ストアからの初期読み込みが完了したか。`true` になるまで永続化処理は走らせない。
  loaded: boolean;
  /// 起動時に読み込まれた config。null の場合は初回起動またはストア空。
  initialConfig: AppConfig | null;
}

/// アプリ起動時に 1 回だけ config を読み込む。後続の保存は `persistConfig` を使う。
export function useLoadConfig(): UseConfigResult {
  const [loaded, setLoaded] = useState(false);
  const [initialConfig, setInitialConfig] = useState<AppConfig | null>(null);

  useEffect(() => {
    let mounted = true;
    (async () => {
      try {
        const store = await getStore();
        const cfg = await store.get<AppConfig>(KEY);
        if (mounted && cfg) setInitialConfig(cfg);
      } catch (e) {
        // 破損 / 不在時は黙ってデフォルトにフォールバック
        console.error("config load failed", e);
      } finally {
        if (mounted) setLoaded(true);
      }
    })();
    return () => {
      mounted = false;
    };
  }, []);

  return { loaded, initialConfig };
}

/// 渡された AppConfig をストアに書き込んで `save()` で永続化する。
/// 呼び出し側でデバウンスする想定。
export async function persistConfig(cfg: AppConfig): Promise<void> {
  try {
    const store = await getStore();
    await store.set(KEY, cfg);
    await store.save();
  } catch (e) {
    console.error("config save failed", e);
  }
}
