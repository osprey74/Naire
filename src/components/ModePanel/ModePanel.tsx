import RegexMode from "./advanced/RegexMode";
import WildcardMode from "./advanced/WildcardMode";
import CharConvertMode from "./advanced/CharConvertMode";
import BuiltinMenu from "./builtin/BuiltinMenu";
import BuiltinParams from "./builtin/BuiltinParams";
import type { BuiltinKind } from "./builtin/builtin-defaults";
import MacroPanel from "./macro/MacroPanel";
import type { BuiltinOp, Macro } from "../../types/rename";
import styles from "./ModePanel.module.css";

type Mode = "builtin" | "advanced" | "macro";
type AdvancedTab = "wildcard" | "regex" | "char_convert";

export interface ModePanelProps {
  mode: Mode;
  onModeChange: (m: Mode) => void;
  advancedTab: AdvancedTab;
  onAdvancedTabChange: (t: AdvancedTab) => void;
  regexSearch: string;
  regexReplace: string;
  onRegexSearchChange: (s: string) => void;
  onRegexReplaceChange: (s: string) => void;
  wildcardSearch: string;
  wildcardReplace: string;
  onWildcardSearchChange: (s: string) => void;
  onWildcardReplaceChange: (s: string) => void;
  charFrom: string;
  charTo: string;
  onCharFromChange: (s: string) => void;
  onCharToChange: (s: string) => void;
  builtinOp: BuiltinOp | null;
  onBuiltinKindSelect: (kind: BuiltinKind) => void;
  onBuiltinOpChange: (op: BuiltinOp) => void;
  macros: Macro[];
  currentMacroId: string | null;
  macroStepIndex: number;
  macroHasItems: boolean;
  onMacroSelect: (id: string | null) => void;
  onMacroCreateNew: () => void;
  onMacroEdit: (id: string) => void;
  onMacroStepForward: () => void;
  onMacroApplyAll: () => void;
  onMacroReset: () => void;
}

export default function ModePanel(props: ModePanelProps) {
  const {
    mode,
    onModeChange,
    advancedTab,
    onAdvancedTabChange,
    regexSearch,
    regexReplace,
    onRegexSearchChange,
    onRegexReplaceChange,
    wildcardSearch,
    wildcardReplace,
    onWildcardSearchChange,
    onWildcardReplaceChange,
    charFrom,
    charTo,
    onCharFromChange,
    onCharToChange,
    builtinOp,
    onBuiltinKindSelect,
    onBuiltinOpChange,
    macros,
    currentMacroId,
    macroStepIndex,
    macroHasItems,
    onMacroSelect,
    onMacroCreateNew,
    onMacroEdit,
    onMacroStepForward,
    onMacroApplyAll,
    onMacroReset,
  } = props;

  return (
    <div className={styles.panel}>
      <div className={styles.tabs}>
        <button
          type="button"
          className={`${styles.tab} ${mode === "builtin" ? styles.active : ""}`}
          onClick={() => onModeChange("builtin")}
        >
          定型
        </button>
        <button
          type="button"
          className={`${styles.tab} ${mode === "advanced" ? styles.active : ""}`}
          onClick={() => onModeChange("advanced")}
        >
          高度な
        </button>
        <button
          type="button"
          className={`${styles.tab} ${mode === "macro" ? styles.active : ""}`}
          onClick={() => onModeChange("macro")}
        >
          マクロ
        </button>
      </div>

      <div className={styles.body}>
        {mode === "builtin" && (
          <div className={styles.builtinLayout}>
            <BuiltinMenu
              selectedKind={builtinOp?.type ?? null}
              onSelect={onBuiltinKindSelect}
            />
            {builtinOp && (
              <BuiltinParams op={builtinOp} onChange={onBuiltinOpChange} />
            )}
            {!builtinOp && (
              <div className={styles.placeholder}>
                上のメニューから定型を選択してください
              </div>
            )}
          </div>
        )}
        {mode === "advanced" && (
          <div>
            <div className={styles.subTabs}>
              <button
                type="button"
                className={`${styles.subTab} ${
                  advancedTab === "regex" ? styles.active : ""
                }`}
                onClick={() => onAdvancedTabChange("regex")}
              >
                正規表現
              </button>
              <button
                type="button"
                className={`${styles.subTab} ${
                  advancedTab === "wildcard" ? styles.active : ""
                }`}
                onClick={() => onAdvancedTabChange("wildcard")}
              >
                ワイルドカード
              </button>
              <button
                type="button"
                className={`${styles.subTab} ${
                  advancedTab === "char_convert" ? styles.active : ""
                }`}
                onClick={() => onAdvancedTabChange("char_convert")}
              >
                文字変換
              </button>
            </div>
            {advancedTab === "regex" && (
              <RegexMode
                search={regexSearch}
                replace={regexReplace}
                onSearchChange={onRegexSearchChange}
                onReplaceChange={onRegexReplaceChange}
              />
            )}
            {advancedTab === "wildcard" && (
              <WildcardMode
                search={wildcardSearch}
                replace={wildcardReplace}
                onSearchChange={onWildcardSearchChange}
                onReplaceChange={onWildcardReplaceChange}
              />
            )}
            {advancedTab === "char_convert" && (
              <CharConvertMode
                from={charFrom}
                to={charTo}
                onFromChange={onCharFromChange}
                onToChange={onCharToChange}
              />
            )}
          </div>
        )}
        {mode === "macro" && (
          <MacroPanel
            macros={macros}
            currentMacroId={currentMacroId}
            stepIndex={macroStepIndex}
            hasItems={macroHasItems}
            onSelect={onMacroSelect}
            onCreateNew={onMacroCreateNew}
            onEdit={onMacroEdit}
            onStepForward={onMacroStepForward}
            onApplyAll={onMacroApplyAll}
            onReset={onMacroReset}
          />
        )}
      </div>
    </div>
  );
}

export type { Mode, AdvancedTab };
