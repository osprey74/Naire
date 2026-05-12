import { useRef } from "react";
import SupportButton from "../../SupportButton/SupportButton";
import styles from "./RegexMode.module.css";

export interface WildcardModeProps {
  search: string;
  replace: string;
  onSearchChange: (s: string) => void;
  onReplaceChange: (s: string) => void;
}

export default function WildcardMode(props: WildcardModeProps) {
  const { search, replace, onSearchChange, onReplaceChange } = props;
  const searchRef = useRef<HTMLInputElement>(null);
  const replaceRef = useRef<HTMLInputElement>(null);

  return (
    <div className={styles.mode}>
      <div className={styles.field}>
        <div className={styles.header}>
          <span className={styles.label}>検索（ワイルドカード）</span>
          <SupportButton
            context="wildcard.search"
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
          placeholder="例: *.jpg"
          spellCheck={false}
        />
      </div>
      <div className={styles.field}>
        <div className={styles.header}>
          <span className={styles.label}>置換</span>
          <SupportButton
            context="wildcard.replace"
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
          placeholder={String.raw`\t.png など`}
          spellCheck={false}
        />
      </div>
      <div className={styles.hint}>
        <code>?</code> = 任意の 1 文字、<code>*</code> = 任意の文字列。
        パターンはファイル名全体に対してマッチ（前方・末尾アンカー）。
        キャプチャは生成されないため <code>\1</code>〜<code>\9</code> は不可。
        マッチ全体は <code>\0</code>、stem は <code>\t</code>、拡張子は{" "}
        <code>\e</code> で参照。
      </div>
    </div>
  );
}
