import { cloneElement, useId, type ReactElement } from "react";
import type { BuiltinOp, DeletePattern } from "../../../types/rename";
import styles from "./BuiltinParams.module.css";

export interface BuiltinParamsProps {
  op: BuiltinOp;
  onChange: (op: BuiltinOp) => void;
}

/// 各 variant の編集 UI を分岐レンダする。各 case 内で `op` は narrow されるので、
/// `onChange({ ...op, field: value })` で型安全に部分更新できる。
export default function BuiltinParams({ op, onChange }: BuiltinParamsProps) {
  switch (op.type) {
    case "add_seq_str":
      return (
        <div className={styles.params}>
          <Field label="プレフィックス">
            <input
              type="text"
              className={styles.text}
              value={op.prefix}
              onChange={(e) => onChange({ ...op, prefix: e.target.value })}
              spellCheck={false}
            />
          </Field>
          <Field label="サフィックス">
            <input
              type="text"
              className={styles.text}
              value={op.suffix}
              onChange={(e) => onChange({ ...op, suffix: e.target.value })}
              spellCheck={false}
            />
          </Field>
          <Row>
            <NumField
              label="桁数"
              value={op.digits}
              onChange={(v) => onChange({ ...op, digits: v })}
              min={1}
            />
            <NumField
              label="開始"
              value={op.start}
              onChange={(v) => onChange({ ...op, start: v })}
              min={0}
            />
            <NumField
              label="ステップ"
              value={op.step}
              onChange={(v) => onChange({ ...op, step: v })}
              min={1}
            />
          </Row>
          <Hint>
            結果は <code>{`{prefix}{seq}{suffix}{.ext}`}</code>（stem を置換）
          </Hint>
        </div>
      );

    case "add_datetime":
      return (
        <div className={styles.params}>
          <Field label="日時書式">
            <input
              type="text"
              className={styles.text}
              value={op.format}
              onChange={(e) => onChange({ ...op, format: e.target.value })}
              placeholder="\Y\m\d"
              spellCheck={false}
            />
          </Field>
          <Field label="位置">
            <select
              aria-label="日時の位置"
              className={styles.select}
              value={op.position}
              onChange={(e) =>
                onChange({ ...op, position: e.target.value as "prefix" | "suffix" })
              }
            >
              <option value="prefix">先頭</option>
              <option value="suffix">末尾</option>
            </select>
          </Field>
          <Hint>
            <code>{"\\Y \\m \\d \\H \\M \\S \\#m"}</code> など。ファイル mtime ベース
          </Hint>
        </div>
      );

    case "add_folder_name":
      return (
        <div className={styles.params}>
          <Field label="位置">
            <select
              className={styles.select}
              value={op.position}
              onChange={(e) =>
                onChange({ ...op, position: e.target.value as "prefix" | "suffix" })
              }
            >
              <option value="prefix">先頭</option>
              <option value="suffix">末尾</option>
            </select>
          </Field>
        </div>
      );

    case "add_folder_seq":
      return (
        <div className={styles.params}>
          <Row>
            <NumField
              label="桁数"
              value={op.digits}
              onChange={(v) => onChange({ ...op, digits: v })}
              min={1}
            />
            <NumField
              label="開始"
              value={op.start}
              onChange={(v) => onChange({ ...op, start: v })}
              min={0}
            />
            <NumField
              label="ステップ"
              value={op.step}
              onChange={(v) => onChange({ ...op, step: v })}
              min={1}
            />
          </Row>
          <Hint>
            結果は <code>{`{folder}{seq}{.ext}`}</code>
          </Hint>
        </div>
      );

    case "truncate_from_start":
      return (
        <div className={styles.params}>
          <NumField
            label="削除文字数 n"
            value={op.n}
            onChange={(v) => onChange({ ...op, n: v })}
            min={0}
          />
        </div>
      );

    case "truncate_from_end":
      return (
        <div className={styles.params}>
          <NumField
            label="削除文字数 n"
            value={op.n}
            onChange={(v) => onChange({ ...op, n: v })}
            min={0}
          />
        </div>
      );

    case "delete_chars":
      return (
        <div className={styles.params}>
          <Field label="起点">
            <select
              className={styles.select}
              value={op.from}
              onChange={(e) =>
                onChange({ ...op, from: e.target.value as "start" | "end" })
              }
            >
              <option value="start">先頭から</option>
              <option value="end">末尾から</option>
            </select>
          </Field>
          <Row>
            <NumField
              label="開始位置"
              value={op.offset}
              onChange={(v) => onChange({ ...op, offset: v })}
              min={0}
            />
            <NumField
              label="文字数"
              value={op.count}
              onChange={(v) => onChange({ ...op, count: v })}
              min={0}
            />
          </Row>
        </div>
      );

    case "delete_before":
      return (
        <div className={styles.params}>
          <Field label="起点">
            <select
              className={styles.select}
              value={op.from}
              onChange={(e) =>
                onChange({ ...op, from: e.target.value as "start" | "end" })
              }
            >
              <option value="start">先頭から</option>
              <option value="end">末尾から</option>
            </select>
          </Field>
          <NumField
            label="n"
            value={op.n}
            onChange={(v) => onChange({ ...op, n: v })}
            min={0}
          />
        </div>
      );

    case "delete_pattern":
      return (
        <div className={styles.params}>
          <Field label="パターン">
            <select
              className={styles.select}
              value={op.pattern}
              onChange={(e) =>
                onChange({ ...op, pattern: e.target.value as DeletePattern })
              }
            >
              <option value="copy_num">"コピー (N)" / "- Copy (N)"</option>
              <option value="copy_num_vista">"ーコピー (N)" (Vista)</option>
              <option value="shortcut">"へのショートカット" / "- Shortcut"</option>
              <option value="shortcut_vista">"ーショートカット" (Vista)</option>
              <option value="bracket_content">括弧とその中身（全種）</option>
              <option value="num_kagi_or_round">【数字】または（数字）</option>
              <option value="num_kagi">【数字】</option>
              <option value="num_round">（数字）</option>
              <option value="num_lenticular">〔数字〕</option>
            </select>
          </Field>
        </div>
      );

    case "make_83":
      return (
        <Hint>パラメータなし — stem 最大 8 文字、拡張子最大 3 文字に切り詰め</Hint>
      );

    case "case_convert":
      return (
        <div className={styles.params}>
          <Field label="変換">
            <select
              className={styles.select}
              value={op.conversion}
              onChange={(e) =>
                onChange({ ...op, conversion: e.target.value as typeof op.conversion })
              }
            >
              <option value="capitalize">語頭を大文字</option>
              <option value="upper">小文字 → 大文字</option>
              <option value="lower">大文字 → 小文字</option>
            </select>
          </Field>
          <SkipExtCheck
            checked={op.skip_ext}
            onChange={(v) => onChange({ ...op, skip_ext: v })}
          />
        </div>
      );

    case "kana_convert":
      return (
        <div className={styles.params}>
          <Field label="変換">
            <select
              className={styles.select}
              value={op.conversion}
              onChange={(e) =>
                onChange({ ...op, conversion: e.target.value as typeof op.conversion })
              }
            >
              <option value="hira_to_kata">ひらがな → カタカナ</option>
              <option value="kata_to_hira">カタカナ → ひらがな</option>
              <option value="hankaku_kata_to_zenkaku">半角カナ → 全角カナ</option>
              <option value="zenkaku_kata_to_hankaku">全角カナ → 半角カナ</option>
            </select>
          </Field>
          <SkipExtCheck
            checked={op.skip_ext}
            onChange={(v) => onChange({ ...op, skip_ext: v })}
          />
        </div>
      );

    case "width_convert":
      return (
        <div className={styles.params}>
          <Field label="変換">
            <select
              className={styles.select}
              value={op.conversion}
              onChange={(e) =>
                onChange({ ...op, conversion: e.target.value as typeof op.conversion })
              }
            >
              <option value="to_hankaku">全角 → 半角</option>
              <option value="to_zenkaku">半角 → 全角</option>
            </select>
          </Field>
          <SkipExtCheck
            checked={op.skip_ext}
            onChange={(v) => onChange({ ...op, skip_ext: v })}
          />
        </div>
      );

    case "clear_diacritics":
      return (
        <div className={styles.params}>
          <SkipExtCheck
            checked={op.skip_ext}
            onChange={(v) => onChange({ ...op, skip_ext: v })}
          />
        </div>
      );

    case "remove_voiced_mark":
      return (
        <div className={styles.params}>
          <SkipExtCheck
            checked={op.skip_ext}
            onChange={(v) => onChange({ ...op, skip_ext: v })}
          />
        </div>
      );

    case "string_replace":
      return (
        <div className={styles.params}>
          <Field label="検索">
            <input
              type="text"
              className={styles.text}
              value={op.search}
              onChange={(e) => onChange({ ...op, search: e.target.value })}
              spellCheck={false}
            />
          </Field>
          <Field label="置換">
            <input
              type="text"
              className={styles.text}
              value={op.replace}
              onChange={(e) => onChange({ ...op, replace: e.target.value })}
              spellCheck={false}
            />
          </Field>
        </div>
      );

    case "number_pad":
      return (
        <div className={styles.params}>
          <Field label="起点">
            <select
              className={styles.select}
              value={op.from}
              onChange={(e) =>
                onChange({ ...op, from: e.target.value as "start" | "end" })
              }
            >
              <option value="start">先頭から</option>
              <option value="end">末尾から</option>
            </select>
          </Field>
          <Row>
            <NumField
              label="n 番目"
              value={op.n}
              onChange={(v) => onChange({ ...op, n: v })}
              min={1}
            />
            <NumField
              label="桁数"
              value={op.digits}
              onChange={(v) => onChange({ ...op, digits: v })}
              min={1}
            />
          </Row>
        </div>
      );

    case "number_adjust":
      return (
        <div className={styles.params}>
          <Field label="起点">
            <select
              className={styles.select}
              value={op.from}
              onChange={(e) =>
                onChange({ ...op, from: e.target.value as "start" | "end" })
              }
            >
              <option value="start">先頭から</option>
              <option value="end">末尾から</option>
            </select>
          </Field>
          <Row>
            <NumField
              label="n 番目"
              value={op.n}
              onChange={(v) => onChange({ ...op, n: v })}
              min={1}
            />
            <SignedNumField
              label="増減 delta"
              value={op.delta}
              onChange={(v) => onChange({ ...op, delta: v })}
            />
          </Row>
        </div>
      );

    case "ext_convert":
      return (
        <div className={styles.params}>
          <Field label="変換">
            <select
              className={styles.select}
              value={op.conversion}
              onChange={(e) =>
                onChange({ ...op, conversion: e.target.value as typeof op.conversion })
              }
            >
              <option value="lower">大文字 → 小文字</option>
              <option value="upper">小文字 → 大文字</option>
            </select>
          </Field>
        </div>
      );

    case "ext_delete":
      return <Hint>パラメータなし — 拡張子を削除</Hint>;

    case "ext_add":
      return (
        <div className={styles.params}>
          <Field label="拡張子">
            <input
              type="text"
              className={styles.text}
              value={op.ext}
              onChange={(e) => onChange({ ...op, ext: e.target.value })}
              placeholder="bak"
              spellCheck={false}
            />
          </Field>
          <Hint>
            先頭の <code>.</code> は省略可
          </Hint>
        </div>
      );

    case "ext_replace":
      return (
        <div className={styles.params}>
          <Field label="新しい拡張子">
            <input
              type="text"
              className={styles.text}
              value={op.ext}
              onChange={(e) => onChange({ ...op, ext: e.target.value })}
              placeholder="md"
              spellCheck={false}
            />
          </Field>
          <Hint>
            先頭の <code>.</code> は省略可
          </Hint>
        </div>
      );
  }
}

// ── 共通の小コンポーネント ─────────────────────────────────────

function Field({
  label,
  children,
}: {
  label: string;
  children: ReactElement<{ id?: string }>;
}) {
  // useId で生成した一意 id を子要素に注入し、`htmlFor` と関連付ける。
  // a11y リンタが `<label>` 包含のみでは label 関連付けを認識しないため、
  // 明示的に htmlFor/id を張る。
  const id = useId();
  return (
    <div className={styles.field}>
      <label htmlFor={id} className={styles.label}>
        {label}
      </label>
      {cloneElement(children, { id })}
    </div>
  );
}

function Row({ children }: { children: React.ReactNode }) {
  return <div className={styles.row}>{children}</div>;
}

function Hint({ children }: { children: React.ReactNode }) {
  return <div className={styles.hint}>{children}</div>;
}

function NumField(props: {
  label: string;
  value: number;
  onChange: (v: number) => void;
  min?: number;
}) {
  const { label, value, onChange, min = 0 } = props;
  return (
    <Field label={label}>
      <input
        type="number"
        aria-label={label}
        className={styles.num}
        min={min}
        value={value}
        onChange={(e) => onChange(Math.max(min, Number(e.target.value) || min))}
      />
    </Field>
  );
}

function SignedNumField(props: {
  label: string;
  value: number;
  onChange: (v: number) => void;
}) {
  const { label, value, onChange } = props;
  return (
    <Field label={label}>
      <input
        type="number"
        aria-label={label}
        className={styles.num}
        value={value}
        onChange={(e) => onChange(Number(e.target.value) || 0)}
      />
    </Field>
  );
}

function SkipExtCheck(props: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className={styles.check}>
      <input
        type="checkbox"
        checked={props.checked}
        onChange={(e) => props.onChange(e.target.checked)}
      />
      拡張子を除く
    </label>
  );
}
