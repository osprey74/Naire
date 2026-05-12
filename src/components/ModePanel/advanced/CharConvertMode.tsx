import { useRef } from "react";
import SupportButton from "../../SupportButton/SupportButton";
import styles from "./RegexMode.module.css";

export interface CharConvertModeProps {
  from: string;
  to: string;
  onFromChange: (s: string) => void;
  onToChange: (s: string) => void;
}

export default function CharConvertMode(props: CharConvertModeProps) {
  const { from, to, onFromChange, onToChange } = props;
  const fromRef = useRef<HTMLInputElement>(null);
  const toRef = useRef<HTMLInputElement>(null);

  return (
    <div className={styles.mode}>
      <div className={styles.field}>
        <div className={styles.header}>
          <span className={styles.label}>変換前（from）</span>
          <SupportButton
            context="char.search"
            inputRef={fromRef}
            onValueChange={onFromChange}
          />
        </div>
        <input
          ref={fromRef}
          type="text"
          className={styles.input}
          value={from}
          onChange={(e) => onFromChange(e.target.value)}
          placeholder="例: a-z または 0,i,ii,iii,..."
          spellCheck={false}
        />
      </div>
      <div className={styles.field}>
        <div className={styles.header}>
          <span className={styles.label}>変換後（to）</span>
          <SupportButton
            context="char.replace"
            inputRef={toRef}
            onValueChange={onToChange}
          />
        </div>
        <input
          ref={toRef}
          type="text"
          className={styles.input}
          value={to}
          onChange={(e) => onToChange(e.target.value)}
          placeholder="例: A-Z または 0,1,2,3,..."
          spellCheck={false}
        />
      </div>
      <div className={styles.hint}>
        範囲: <code>a-z</code>（26 要素）/ 列挙:{" "}
        <code>0,i,ii,iii</code>（カンマ区切り）。
        マルチ文字トークンは長いものから貪欲マッチ。
        <code>to</code> が短いときは末尾要素でパディング、空なら削除。
      </div>
    </div>
  );
}
