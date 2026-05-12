# Naire Macro Authoring Reference (v0.1.0)

> A self-contained reference for writing Naire macros, including macros generated
> by LLMs such as Claude / ChatGPT / Gemini. Paste this entire file into the
> assistant before asking it to author a macro and it will have full context.

Naire is a Windows / macOS bulk rename tool with a 4-kind step pipeline:
**regex / wildcard / char_convert / builtin**. A macro is an ordered list of
those steps that is applied to each selected file or folder, one step at a
time. The current name is mutated step by step; the final name is then written
to disk.

---

## 1. Macro JSON format

The file written by **↓ Export** in the macro editor has two shapes — either a
single object (one macro) or an array (many macros). Both are accepted on
import. IDs are regenerated on import so you may set `id` to anything.

```json
{
  "id": "any-string",
  "name": "Display name shown in the dropdown",
  "created_at": "2026-05-12T20:00:00+09:00",
  "updated_at": "2026-05-12T20:00:00+09:00",
  "steps": [
    { "kind": "regex",   "search": "...", "replace": "..." },
    { "kind": "wildcard", "search": "...", "replace": "..." },
    { "kind": "char_convert", "from": "...", "to": "..." },
    { "kind": "builtin",  "op": { "type": "...", "...": "..." } }
  ]
}
```

Required fields: `id`, `name`, `created_at`, `updated_at`, `steps`. The
timestamps must be ISO 8601 strings but their content is not validated.

---

## 2. Step kinds

### 2.1 `regex`

Rust `regex` crate syntax. The pattern matches against the **current name**
(the output of the previous step). `replace_all` is used, so every match is
replaced.

```json
{ "kind": "regex", "search": "^(\\d+)_", "replace": "" }
```

The replace template supports the variables listed in §4.

### 2.2 `wildcard`

`?` and `*` translated to `.` and `.*`, with the whole pattern anchored as
`^...$`. Regex metacharacters (`.` `\` `(` `)` `[` `]` `{` `}` `+` `^` `$` `|`)
are escaped automatically. **Capture groups are not generated**, so `\1`…`\9`
cannot reference the wildcard match — use `\0` (full match = full name) or
`\t` (stem) etc.

```json
{ "kind": "wildcard", "search": "*.jpg", "replace": "\\t.png" }
```

### 2.3 `char_convert`

`tr`-style 1:1 character mapping. The `from` and `to` strings are parsed the
same way: if either contains a comma, it's an enumeration (one token per
comma-separated element); otherwise it's a range scan (`X-Y` expands to all
codepoints from X to Y, anything else is one character).

```json
{ "kind": "char_convert",
  "from": "0,i,ii,iii,iv,v,vi,vii,viii,ix,x",
  "to":   "0,1,2,3,4,5,6,7,8,9,10" }
```

Matching is **greedy from longest token first**, so multi-character tokens
like `viii` win over `v` + `iii`. If `to` is shorter than `from`, the last
`to` element is reused; if `to` is empty, matched characters are deleted.

### 2.4 `builtin`

A single named operation with structured parameters. See §9 for the catalog.

```json
{ "kind": "builtin",
  "op": { "type": "remove_voiced_mark", "skip_ext": true } }
```

---

## 3. Sequence counter (global)

Each application of an advanced (`regex` / `wildcard` / `char_convert`) step
that uses `?` / `??` / `???` / `????` reads the counter value from a global
config supplied at execute time. Builtin ops that use sequences
(`add_seq_str` / `add_folder_seq`) override `start` / `step` / `digits` from
the op itself but still inherit `numbering` and `reset_per_folder` from the
global config.

The global `SequenceConfig` (set in the UI's sequence panel):

| field | meaning |
|---|---|
| `start` | first value (≥ 0) |
| `step` | increment (≥ 1) |
| `numbering` | `"decimal"` / `"hex"` / `"alpha"` |
| `reset_per_folder` | reset to `start` whenever the current folder changes |

Counter advances **only for rows that the step is applied to** (partial
selection skips non-selected rows without consuming the counter).

---

## 4. Replacement variables (`regex` / `wildcard` replace fields)

These are expanded inside the replace template of `regex` / `wildcard`
steps, and inside the `format` argument of the builtin `add_datetime`.

### File variables

| Variable | Expands to |
|---|---|
| `\0` | The current name (full filename, including extension) |
| `\t` | The current title (filename without the final extension) |
| `\e` | The current extension including the leading dot, or empty |
| `\f` | The immediate parent folder name |
| `\F` | The grandparent folder name |
| `\;` | File size in bytes with thousands separators (`1,234,567`) |
| `\:` | File size in bytes without separators (`1234567`) |

`\0` `\t` `\e` reflect the **current step input**; `\f` `\F` come from the
filesystem path and do not change during macro execution.

### Date variables (from file mtime, not wall-clock time)

| Variable | Format |
|---|---|
| `\Y` | 4-digit year |
| `\y` | 2-digit year |
| `\m` | Month 01–12 |
| `\d` | Day 01–31 |
| `\H` | Hour 00–23 |
| `\I` | Hour 01–12 |
| `\M` | Minute 00–59 |
| `\S` | Second 00–59 |
| `\#X` | Strip leading zero from the next date variable (`\#m` → `5` not `05`) |

Not yet implemented (passes through as literal): `\a \A \b \B \p`
(locale-dependent weekday / month / am-pm), and `\u \U \l \L \E`
(case-control).

### Capture groups

| Variable | Meaning |
|---|---|
| `\1` … `\9` | regex capture groups (regex steps only) |

In `wildcard` mode `\1`…`\9` are empty because wildcards do not produce
captures.

### Sequence

| Variable | Width |
|---|---|
| `?` | 1 digit |
| `??` | 2 digits |
| `???` | 3 digits |
| `????` | 4 digits |

Width is minimum width — overflow extends naturally (`??` for value 100 →
`100`). For `numbering: "alpha"` width is ignored entirely (bijective
base-26 grows on its own: A, B, …, Z, AA, AB, …, ZZ, AAA, …).

### Macro-only

| Variable | Meaning |
|---|---|
| `\orig` | The filename at step 0 (macro start) |

Outside macro contexts `\orig` equals `\0`.

### Escapes

| Sequence | Output |
|---|---|
| `\\` | Literal backslash |
| `\?` | Literal `?` (suppresses sequence expansion) |

Any unknown `\X` is passed through as the two literal characters `\X`.

---

## 5. Rust regex caveats

Naire uses Rust's `regex` crate (linear-time, ReDoS-safe). This means a few
classic regex features from Boost.Regex / PCRE / JavaScript are **not
supported**:

| Feature | Naire support | Workaround |
|---|---|---|
| Lookahead `(?=...)` `(?!...)` | ❌ | Capture the matched portion and rewrite it back |
| Lookbehind `(?<=...)` `(?<!...)` | ❌ | Same |
| In-pattern backreference `(\d)\1` | ❌ | Split into multiple macro steps |
| `\<` `\>` word boundaries | ❌ | Use `\b` |
| `\b` `\B` `\A` `\Z` | ✅ | — |
| `(?:...)` non-capturing groups | ✅ | — |
| Lazy quantifiers `*?` `+?` `??` `{m,n}?` | ✅ | — |

The single most common pitfall: writing `\]([^ ])` instead of `\][^ ]` so
the next character is captured back, because Rust regex cannot peek
without consuming.

```jsonc
// Bug: removes the character following "]"
{ "kind": "regex", "search": "\\][^ ]", "replace": "] " }

// Fix: capture and restore
{ "kind": "regex", "search": "\\]([^ ])", "replace": "] \\1" }
```

---

## 6. Wildcard semantics in detail

- `?` matches exactly one character (any Unicode scalar).
- `*` matches zero or more characters.
- Regex metacharacters are escaped automatically — `.zip` matches a literal
  `.zip`, not "any-char + zip".
- The pattern is anchored to the full filename (`^...$`), so `*.jpg` only
  matches names that end with `.jpg`.
- **No capture groups**, so use `\0` for the whole match, `\t` for the stem,
  `\e` for the extension.

---

## 7. Character convert syntax

`from` and `to` are parsed identically.

**Enumeration mode** (any comma → comma-separated tokens):

```json
{ "from": "0,i,ii,iii", "to": "0,1,2,3" }
```

**Range mode** (no comma → range scan):

- `a-z` expands to 26 single-character tokens.
- `あ-ん` expands across the Unicode block U+3042–U+3093 (every codepoint in
  between, including the small kana, dakuten variants, etc.).
- Lone characters are taken as 1-character tokens. `〇一二三` is four
  tokens.
- A `-` not between two characters is treated as a literal `-`.

If lengths differ: extra `to` tokens are ignored; if `to` is shorter the last
`to` element pads the rest; if `to` is empty (`""`), matched characters are
**deleted**.

Greedy match prefers longer `from` tokens, so an enumeration like
`0,i,ii,iii,iv,v,vi,vii,viii,ix,x` resolves `viii` as one token.

---

## 8. Builtin op catalog

Each example shows the JSON shape of `op` inside `{ "kind": "builtin", "op": ... }`.

### Add / replace operations

#### `add_seq_str` — replace stem with `{prefix}{seq}{suffix}`

```json
{ "type": "add_seq_str",
  "prefix": "img_", "suffix": "",
  "digits": 3, "start": 0, "step": 1 }
```

Replaces the stem of each name with the built string and keeps the extension.

#### `add_datetime` — prepend or append a formatted date string

```json
{ "type": "add_datetime",
  "format": "\\Y\\m\\d_",
  "position": "prefix" }
```

`format` is the same template language as regex replace (see §4). `position`
is `"prefix"` (before the whole name) or `"suffix"` (between stem and
extension).

#### `add_folder_name` — prepend or append the parent folder name

```json
{ "type": "add_folder_name", "position": "prefix" }
```

#### `add_folder_seq` — replace stem with `{folder_name}{seq}`

```json
{ "type": "add_folder_seq",
  "digits": 3, "start": 0, "step": 1 }
```

#### `truncate_from_start` / `truncate_from_end`

```json
{ "type": "truncate_from_start", "n": 3 }
{ "type": "truncate_from_end",   "n": 4 }
```

Remove `n` characters from the beginning / end of the **full name**
(extension included).

### Delete operations

#### `delete_chars`

```json
{ "type": "delete_chars",
  "from": "start", "offset": 2, "count": 3 }
```

Removes `count` characters starting `offset` from the named side. `from` is
`"start"` or `"end"`.

#### `delete_before`

```json
{ "type": "delete_before", "from": "start", "n": 3 }
```

`from: "start", n: N` → drop the first N characters.
`from: "end",   n: N` → drop the last N characters.

#### `delete_pattern`

```json
{ "type": "delete_pattern", "pattern": "copy_num" }
```

`pattern` is one of:

| Value | Removes |
|---|---|
| `copy_num` | ` - Copy`, ` - Copy (N)`, ` - コピー`, ` - コピー (N)`, ` コピー (N)` |
| `copy_num_vista` | `ーCopy`, `ーCopy (N)`, `ーコピー`, `ーコピー (N)` (Vista long-dash variant) |
| `shortcut` | ` - Shortcut`, ` - ショートカット`, `へのショートカット` |
| `shortcut_vista` | `ーShortcut`, `ーショートカット` |
| `bracket_content` | Every paired bracket span — `[…]` `(…)` `（…）` `「…」` `『…』` `【…】` `〔…〕` `《…》` and their contents |
| `num_kagi` | `【N】` |
| `num_round` | `（N）` (full-width parens) |
| `num_lenticular` | `〔N〕` |
| `num_kagi_or_round` | either of the above two |

#### `make_83` — truncate stem to 8 chars and extension to 3 chars (DOS 8.3)

```json
{ "type": "make_83" }
```

### Character / case conversions (all support `skip_ext`)

`skip_ext: true` means the conversion is applied to the stem only and the
extension is left untouched.

#### `case_convert`

```json
{ "type": "case_convert",
  "conversion": "lower", "skip_ext": true }
```

`conversion`: `"capitalize"` (word-initial uppercase) / `"upper"` /
`"lower"`.

#### `kana_convert`

```json
{ "type": "kana_convert",
  "conversion": "kata_to_hira", "skip_ext": true }
```

`conversion`:

| Value | Effect |
|---|---|
| `hira_to_kata` | Hiragana → Katakana |
| `kata_to_hira` | Katakana → Hiragana |
| `hankaku_kata_to_zenkaku` | Half-width kana → full-width (with dakuten merging) |
| `zenkaku_kata_to_hankaku` | Full-width kana → half-width (with dakuten decomposition) |

#### `width_convert`

```json
{ "type": "width_convert",
  "conversion": "to_hankaku", "skip_ext": true }
```

`conversion`: `"to_zenkaku"` / `"to_hankaku"` — covers ASCII printable
range Ｕ+0021–Ｕ+007E ↔ U+FF01–U+FF5E (alphanumerics and symbols).

#### `clear_diacritics`

```json
{ "type": "clear_diacritics", "skip_ext": true }
```

NFD-decomposes, drops every combining mark (Latin accents etc.), then NFC.

#### `remove_voiced_mark` — Naire-specific

```json
{ "type": "remove_voiced_mark", "skip_ext": true }
```

Drops `U+3099` and `U+309A` (combining voiced / half-voiced marks) after
NFD decomposition, then NFC. Works for both hiragana and katakana: `ガ` →
`カ`, `ぱ` → `は`.

### String / number / extension utilities

#### `string_replace` — global plain-text replace

```json
{ "type": "string_replace",
  "search": "foo", "replace": "bar" }
```

Not regex — exact substring match, all occurrences.

#### `number_pad`

```json
{ "type": "number_pad",
  "from": "start", "n": 1, "digits": 3 }
```

Finds the `n`-th run of digits from the named side and zero-pads it to
`digits` width. `from`: `"start"` / `"end"`.

#### `number_adjust`

```json
{ "type": "number_adjust",
  "from": "end", "n": 1, "delta": -2 }
```

Finds the `n`-th number from the named side and adds `delta` to it, keeping
the original width via zero-padding. Result is clamped to ≥ 0.

#### `ext_convert`

```json
{ "type": "ext_convert", "conversion": "lower" }
```

`conversion`: `"upper"` / `"lower"`.

#### `ext_delete`

```json
{ "type": "ext_delete" }
```

#### `ext_add`

```json
{ "type": "ext_add", "ext": "bak" }
```

A leading dot is added if absent.

#### `ext_replace`

```json
{ "type": "ext_replace", "ext": "md" }
```

---

## 9. Worked examples

### 9.1 Bulk extension change `.jpg → .png`

Single regex step:

```json
{
  "id": "x", "name": "jpg → png",
  "created_at": "", "updated_at": "",
  "steps": [
    { "kind": "regex", "search": "\\.jpg$", "replace": ".png" }
  ]
}
```

Or with a builtin:

```json
{ "kind": "builtin", "op": { "type": "ext_replace", "ext": "png" } }
```

### 9.2 Strip `(N)` copy markers

```json
{
  "id": "x", "name": "Clean Windows copy markers",
  "created_at": "", "updated_at": "",
  "steps": [
    { "kind": "builtin",
      "op": { "type": "delete_pattern", "pattern": "copy_num" } }
  ]
}
```

### 9.3 Author sort key (folder mode)

Given a folder named `[author] title`, prepend a lowercased / dakuten-stripped
first character of the author.

```json
{
  "id": "x", "name": "Author sort key",
  "created_at": "", "updated_at": "",
  "steps": [
    { "kind": "regex", "search": "^\\[(.).*", "replace": "\\1" },
    { "kind": "builtin",
      "op": { "type": "kana_convert", "conversion": "kata_to_hira", "skip_ext": true } },
    { "kind": "builtin",
      "op": { "type": "remove_voiced_mark", "skip_ext": true } },
    { "kind": "builtin",
      "op": { "type": "width_convert", "conversion": "to_hankaku", "skip_ext": true } },
    { "kind": "builtin",
      "op": { "type": "case_convert", "conversion": "lower", "skip_ext": true } },
    { "kind": "regex", "search": "^(.*)$", "replace": "\\1 \\orig" }
  ]
}
```

Trace for `[ガタタン研究会] 芦別名物`:

| Step | current |
|---|---|
| 0 (initial) | `[ガタタン研究会] 芦別名物` |
| 1 | `ガ` |
| 2 | `が` |
| 3 | `か` |
| 4 | `か` (unchanged) |
| 5 | `か` (unchanged) |
| 6 | `か [ガタタン研究会] 芦別名物` |

### 9.4 Numbered batch rename with folder prefix

```json
{
  "id": "x", "name": "folder + 3-digit seq",
  "created_at": "", "updated_at": "",
  "steps": [
    { "kind": "builtin",
      "op": { "type": "add_folder_seq",
              "digits": 3, "start": 1, "step": 1 } }
  ]
}
```

Filenames are replaced with `{folder_name}001.{ext}`, `{folder_name}002.{ext}`, …

### 9.5 Add `_archived` before the extension

```json
{
  "id": "x", "name": "archive suffix",
  "created_at": "", "updated_at": "",
  "steps": [
    { "kind": "regex",
      "search": "^(.+)(\\.[^.]+)$",
      "replace": "\\1_archived\\2" }
  ]
}
```

For names without an extension this regex won't match and the file is
skipped, which is usually the desired behaviour.

---

## 10. Common pitfalls

1. **Lookaround substitute**: write `(...)` and reuse with `\1` instead of
   `(?=...)`. See §5.
2. **Unescaped `.`**: `\.zip$` matches a literal `.zip` extension; `.zip$`
   matches any-char + `zip` at the end of the name.
3. **Folder mode vs file mode** matters: `\f` is the **parent** folder name
   regardless of mode, so when renaming the folders themselves, do **not**
   include a `.* → \f` step — operate directly on the current name.
4. **`alpha` numbering ignores `digits`**. It is bijective base-26 (`A`,
   `B`, …, `Z`, `AA`, …, `ZZ`, `AAA`, …) — there is no padding.
5. **`\orig` is macro-only**. In a single-step rename it equals `\0`, but
   relying on that is a smell — use `\0` when you mean "current name".
6. **`replace_all` is global** — every match in the string is replaced. If
   you only want the first, anchor with `^` / make the regex more specific.
7. **Empty `replace` is allowed** and means "delete the matched span".
8. **Conflicting renames are rejected at execute time**. If two rows would
   end up with the same target name, the whole batch aborts before any file
   is touched.
9. **Chains are rejected**. `a→b` and `b→c` in the same batch errors out
   (the spec keeps Naire from running unsafe rename sequences). Either
   split into two executions or rewrite the regex.

---

## 11. Prompt template for AI authors

When asking an LLM to write a Naire macro, paste the whole of this document,
then ask roughly:

```
You have read the Naire macro reference above. Please output a single Macro
JSON object (the object form, not the array form) for the following task:

  <describe the task in plain language>

Rules:
- Set id to any short placeholder; it will be regenerated on import.
- Use ISO 8601 timestamps for created_at / updated_at.
- Prefer builtin ops over regex when a builtin matches the intent.
- Remember Rust regex has no lookaround; use capture-and-restore.
- Escape regex meta-characters in literal strings.
- For wildcards, use \t / \e for stem / extension since captures are absent.
- If a step kind isn't required, omit it; only include the steps actually used.

Return ONLY the JSON, in a fenced code block, with no commentary.
```

The user can then save the JSON to a `.json` file and load it with
**↑ Import** in Naire's macro panel.
