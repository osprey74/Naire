import type { RenameStep } from "../../../types/rename";
import { initialOp } from "../builtin/builtin-defaults";

export type StepKind = RenameStep["kind"];

/// 各 kind の初期状態を生成する。MacroEditor の [+ ステップ追加] や、
/// kind を切り替えたときに使う。
export function initialStep(kind: StepKind): RenameStep {
  switch (kind) {
    case "regex":
      return { kind: "regex", search: "", replace: "" };
    case "wildcard":
      return { kind: "wildcard", search: "", replace: "" };
    case "char_convert":
      return { kind: "char_convert", from: "", to: "" };
    case "builtin":
      return { kind: "builtin", op: initialOp("case_convert") };
  }
}

export function describeStep(step: RenameStep): string {
  switch (step.kind) {
    case "regex":
      return `[正規表現] ${step.search || "(未設定)"} → ${step.replace || "(空)"}`;
    case "wildcard":
      return `[ワイルドカード] ${step.search || "(未設定)"} → ${step.replace || "(空)"}`;
    case "char_convert":
      return `[文字変換] ${step.from || "(未設定)"} → ${step.to || "(空)"}`;
    case "builtin":
      return `[定型] ${step.op.type}`;
  }
}
