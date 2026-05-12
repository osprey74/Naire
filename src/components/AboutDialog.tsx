import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import styles from "./AboutDialog.module.css";

const ICON_ATTRIBUTION_URL = "https://www.flaticon.com/free-icons/rename";

export interface AboutDialogProps {
  onClose: () => void;
}

export default function AboutDialog({ onClose }: AboutDialogProps) {
  const [version, setVersion] = useState<string>("");

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion(""));
  }, []);

  const openAttribution = (e: React.MouseEvent<HTMLAnchorElement>) => {
    e.preventDefault();
    openUrl(ICON_ATTRIBUTION_URL).catch(() => {});
  };

  return (
    <div className={styles.backdrop} onClick={onClose}>
      <div className={styles.modal} onClick={(e) => e.stopPropagation()}>
        <div className={styles.header}>
          <h2 className={styles.title}>Naire について</h2>
          <button
            type="button"
            className={styles.closeBtn}
            onClick={onClose}
            aria-label="閉じる"
          >
            ×
          </button>
        </div>

        <div className={styles.body}>
          <div className={styles.appName}>Naire</div>
          <div className={styles.version}>
            バージョン {version || "—"}
          </div>
          <div className={styles.tagline}>一括リネームツール</div>

          <div className={styles.divider} />

          <div className={styles.copyright}>© 2026 osprey74</div>

          <div className={styles.attribution}>
            <div className={styles.attributionLabel}>アプリアイコン</div>
            <a
              href={ICON_ATTRIBUTION_URL}
              title="rename icons"
              className={styles.link}
              onClick={openAttribution}
            >
              Rename icons created by Freepik - Flaticon
            </a>
          </div>
        </div>

        <div className={styles.footer}>
          <button type="button" className={styles.btn} onClick={onClose}>
            閉じる
          </button>
        </div>
      </div>
    </div>
  );
}
