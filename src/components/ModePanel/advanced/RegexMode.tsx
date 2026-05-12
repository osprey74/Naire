import { useRef } from "react";
import SupportButton from "../../SupportButton/SupportButton";
import styles from "./RegexMode.module.css";

export interface RegexModeProps {
  search: string;
  replace: string;
  onSearchChange: (s: string) => void;
  onReplaceChange: (s: string) => void;
}

export default function RegexMode(props: RegexModeProps) {
  const { search, replace, onSearchChange, onReplaceChange } = props;
  const searchRef = useRef<HTMLInputElement>(null);
  const replaceRef = useRef<HTMLInputElement>(null);

  return (
    <div className={styles.mode}>
      <div className={styles.field}>
        <div className={styles.header}>
          <span className={styles.label}>検索</span>
          <SupportButton
            context="regex.search"
            inputRef={searchRef}
            onValueChange={onSearchChange}
          />
        </div>
        <input
          ref={searchRef}
          type="text"
          className={styles.input}
          value={search}
          onChange={(e) => onSearchChange(e.target.value)}
          placeholder="正規表現パターン"
          spellCheck={false}
        />
      </div>
      <div className={styles.field}>
        <div className={styles.header}>
          <span className={styles.label}>置換</span>
          <SupportButton
            context="regex.replace"
            inputRef={replaceRef}
            onValueChange={onReplaceChange}
          />
        </div>
        <input
          ref={replaceRef}
          type="text"
          className={styles.input}
          value={replace}
          onChange={(e) => onReplaceChange(e.target.value)}
          placeholder={String.raw`\1 \t \e \f など`}
          spellCheck={false}
        />
      </div>
      <div className={styles.hint}>
        <code>\1</code>-<code>\9</code> = キャプチャ、<code>\0</code> = 元名、
        <code>\t</code> = 拡張子除く、<code>\e</code> = 拡張子、
        <code>\f</code> = フォルダ名、<code>\F</code> = 親フォルダ名
      </div>
    </div>
  );
}
