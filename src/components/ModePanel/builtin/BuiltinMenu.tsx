import { useState } from "react";
import { BUILTIN_CATEGORIES, type BuiltinKind } from "./builtin-defaults";
import styles from "./BuiltinMenu.module.css";

export interface BuiltinMenuProps {
  selectedKind: BuiltinKind | null;
  onSelect: (kind: BuiltinKind) => void;
}

export default function BuiltinMenu(props: BuiltinMenuProps) {
  const { selectedKind, onSelect } = props;
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  const toggle = (title: string) =>
    setCollapsed((c) => ({ ...c, [title]: !c[title] }));

  return (
    <div className={styles.menu}>
      {BUILTIN_CATEGORIES.map((cat) => {
        const isCollapsed = collapsed[cat.title] ?? false;
        return (
          <div key={cat.title} className={styles.category}>
            <button
              type="button"
              className={styles.categoryHeader}
              onClick={() => toggle(cat.title)}
            >
              <span className={styles.chevron}>{isCollapsed ? "▶" : "▼"}</span>
              {cat.title}
              <span className={styles.count}>({cat.items.length})</span>
            </button>
            {!isCollapsed && (
              <div className={styles.items}>
                {cat.items.map((item) => (
                  <button
                    key={item.kind}
                    type="button"
                    className={`${styles.item} ${
                      selectedKind === item.kind ? styles.active : ""
                    }`}
                    onClick={() => onSelect(item.kind)}
                  >
                    {item.label}
                  </button>
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
