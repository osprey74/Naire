import { useEffect, useRef, useState } from "react";
import { insertAtCursor } from "./insert-at-cursor";
import { SUPPORT_MENUS, type SupportContext } from "./support-items";
import styles from "./SupportButton.module.css";

interface InputLike {
  current: HTMLInputElement | HTMLTextAreaElement | null;
}

export interface SupportButtonProps {
  context: SupportContext;
  inputRef: InputLike;
  onValueChange: (value: string) => void;
}

export default function SupportButton(props: SupportButtonProps) {
  const { context, inputRef, onValueChange } = props;
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const groups = SUPPORT_MENUS[context] ?? [];

  useEffect(() => {
    if (!open) return;
    const onDocMouseDown = (e: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(e.target as Node)
      ) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onDocMouseDown);
    return () => document.removeEventListener("mousedown", onDocMouseDown);
  }, [open]);

  if (groups.length === 0) return null;

  const handleInsert = (text: string) => {
    const input = inputRef.current;
    if (!input) {
      setOpen(false);
      return;
    }
    const { value, cursor } = insertAtCursor(input, text);
    onValueChange(value);
    setOpen(false);
    requestAnimationFrame(() => {
      input.focus();
      input.setSelectionRange(cursor, cursor);
    });
  };

  return (
    <div ref={containerRef} className={styles.wrap}>
      <button
        type="button"
        className={styles.toggle}
        onClick={() => setOpen((v) => !v)}
        title="挿入候補"
      >
        サポート ▶
      </button>
      {open && (
        <div className={styles.menu}>
          {groups.map((group) => (
            <div key={group.title} className={styles.group}>
              <div className={styles.groupTitle}>{group.title}</div>
              {group.items.map((item) => (
                <button
                  key={item.label}
                  type="button"
                  className={styles.item}
                  onClick={() => handleInsert(item.insert)}
                >
                  <span className={styles.itemLabel}>{item.label}</span>
                  <span className={styles.itemHint}>{item.insert}</span>
                </button>
              ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
