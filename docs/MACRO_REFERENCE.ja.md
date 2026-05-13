# Naire マクロ作成リファレンス（v0.1.0）

> Naire のマクロを記述するための自己完結型のリファレンスです。Claude / ChatGPT
> / Gemini などの AI に貼り付けるだけでマクロ生成に必要な仕様情報をすべて渡せます。

Naire は Windows / macOS 向けの一括ファイル / フォルダリネームツールで、
4 種類のステップ（**regex / wildcard / char_convert / builtin**）からなる
順序付きステップリストを「マクロ」として保存・再生できます。マクロは
対象（選択中の各ファイルまたはフォルダ）に対し、1 ステップずつ
`current_name` を書き換えていきます。最終結果が確定したらディスクへ反映します。

---

## 1. マクロ JSON 形式

マクロエディタの **↓ エクスポート** で書き出される JSON は 2 形式あります:
**単一オブジェクト**（1 マクロ）と **配列**（複数マクロ）。インポート時に
どちらも受け付けます。`id` はインポート時に再生成されるため、任意の文字列で
構いません。

```json
{
  "id": "任意の文字列",
  "name": "ドロップダウンに表示される名前",
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

必須フィールド: `id` / `name` / `created_at` / `updated_at` / `steps`。
タイムスタンプは ISO 8601 文字列ですが内容は検証されません。

---

## 2. ステップ種別

### 2.1 `regex`

Rust の `regex` クレート構文。パターンは**現在の名前**（直前ステップの出力）
に対してマッチされ、`replace_all` が呼ばれるためすべてのマッチが置換されます。

```json
{ "kind": "regex", "search": "^(\\d+)_", "replace": "" }
```

置換テンプレートでは §4 の変数が展開されます。

### 2.2 `wildcard`

`?` `*` を `.` `.*` に変換し、全体を `^...$` でアンカーします。正規表現の
メタ文字（`.` `\` `(` `)` `[` `]` `{` `}` `+` `^` `$` `|`）は自動エスケープ
されます。**キャプチャグループは生成されない**ので、`\1`〜`\9` は使えません。
代わりに `\0`（全マッチ＝全名）や `\t`（拡張子除く）を使ってください。

```json
{ "kind": "wildcard", "search": "*.jpg", "replace": "\\t.png" }
```

### 2.3 `char_convert`

`tr` 風の 1:1 文字マッピング。`from` と `to` は同じ構文で解析されます:
カンマを含むなら**列挙**（カンマ区切りのトークン列）、それ以外は**範囲スキャン**
（`X-Y` は X から Y までのコードポイントを展開、それ以外の文字は 1 文字 1 トークン）。

```json
{ "kind": "char_convert",
  "from": "0,i,ii,iii,iv,v,vi,vii,viii,ix,x",
  "to":   "0,1,2,3,4,5,6,7,8,9,10" }
```

マッチは**長いトークン優先の貪欲方式**なので、`viii` は `v` + `iii` ではなく
`viii` 単独で一致します。`to` が `from` より短ければ末尾要素でパディング、
`to` が空（`""`）ならマッチ文字は削除されます。

### 2.4 `builtin`

構造化パラメータを持つ単一の名前付き定型操作。カタログは §8 参照。

```json
{ "kind": "builtin",
  "op": { "type": "remove_voiced_mark", "skip_ext": true } }
```

---

## 3. 連番カウンタ（グローバル）

`?` / `??` / `???` / `????` を含む高度なステップ（`regex` / `wildcard` /
`char_convert`）は、実行時に渡されるグローバル設定からカウンタ値を取得します。
連番を持つ定型 op（`add_seq_str` / `add_folder_seq`）は `start` / `step` /
`digits` を op 自身の値で**上書き**しますが、`numbering` と `reset_per_folder`
はグローバルから継承します。

UI の連番パネルで設定するグローバル `SequenceConfig`:

| フィールド | 意味 |
|---|---|
| `start` | 開始値（0 以上）|
| `step` | 増分（1 以上）|
| `numbering` | `"decimal"` / `"hex"` / `"alpha"` |
| `reset_per_folder` | フォルダが切り替わった時点で `start` に戻す |

**部分選択時はカウンタを消費するのは選択行のみ**で、非選択行はスキップされます
（連番が飛びません）。

---

## 4. 置換変数（`regex` / `wildcard` の置換フィールド）

これらは `regex` / `wildcard` ステップの置換テンプレートおよび定型
`add_datetime` の `format` 引数で展開されます。

### ファイル変数

| 変数 | 展開先 |
|---|---|
| `\0` | 現在の名前（ファイル名、拡張子含む） |
| `\t` | ファイルタイトル（拡張子除く） |
| `\e` | 拡張子（先頭ドット含む。なければ空） |
| `\f` | 所属フォルダ名 |
| `\F` | 親フォルダ名 |
| `\;` | ファイルサイズ（3 桁区切りあり、`1,234,567`）|
| `\:` | ファイルサイズ（区切りなし、`1234567`）|

`\0` `\t` `\e` は**ステップ入力**を参照し、`\f` `\F` はファイルシステムパスから
取得されてマクロ実行中変化しません。

### 日付変数（ファイル mtime ベース、現在時刻ではない）

| 変数 | フォーマット |
|---|---|
| `\Y` | 4 桁西暦 |
| `\y` | 2 桁西暦 |
| `\m` | 月 01–12 |
| `\d` | 日 01–31 |
| `\H` | 時 00–23 |
| `\I` | 時 01–12 |
| `\M` | 分 00–59 |
| `\S` | 秒 00–59 |
| `\#X` | 次の日付変数の先行ゼロを除去（`\#m` → `5`、`05` ではなく）|
| `\a` | 曜日省略名（OS ロケール依存、例 `月` / `Mon`） |
| `\A` | 曜日完全名（OS ロケール依存、例 `月曜日` / `Monday`） |
| `\b` | 月省略名（OS ロケール依存、例 `5月` / `May`） |
| `\B` | 月完全名（OS ロケール依存、例 `5月` / `May`） |
| `\p` | AM/PM 表記（OS ロケール依存、例 `午前` / `AM`） |

ロケール依存変数は OS のロケールを元に `chrono::format_localized` で書式化されます
（POSIX フォールバックあり）。

### 大文字小文字制御

| 変数 | 意味 |
|---|---|
| `\u` | 直後 1 文字を大文字に変換 |
| `\l` | 直後 1 文字を小文字に変換 |
| `\U` | `\E` まで続く文字を全て大文字に変換 |
| `\L` | `\E` まで続く文字を全て小文字に変換 |
| `\E` | `\U` / `\L` の効果を終了 |

変数展開やキャプチャグループの内容にも作用します（例 `\u\1` でキャプチャの先頭 1
文字を大文字化）。日本語など case を持たない文字はそのまま通過します。

### キャプチャグループ

| 変数 | 意味 |
|---|---|
| `\1` 〜 `\9` | 正規表現キャプチャグループ（regex ステップのみ） |

`wildcard` モードではキャプチャが生成されないため `\1`〜`\9` は空文字列に展開されます。

### 連番

| 変数 | 幅 |
|---|---|
| `?` | 1 桁 |
| `??` | 2 桁 |
| `???` | 3 桁 |
| `????` | 4 桁 |

幅は**最小幅**で、値が大きい場合は自然に拡張されます（`??` で値 100 → `100`）。
`numbering: "alpha"` の場合は幅指定が**無視**され、bijective base-26 で自然に
伸びます（A, B, …, Z, AA, AB, …, ZZ, AAA, …）。

### マクロ専用

| 変数 | 意味 |
|---|---|
| `\orig` | マクロ開始時点（step 0）のファイル名 |

マクロ外（単一ステップ実行）では `\orig` は `\0` と同じ値になります。

### エスケープ

| 構文 | 出力 |
|---|---|
| `\\` | リテラルのバックスラッシュ |
| `\?` | リテラルの `?`（連番展開を抑止） |

未対応の `\X` は 2 文字のリテラル `\X` として出力されます。

---

## 5. Rust regex の制限

Naire は Rust の `regex` クレート（線形時間保証・ReDoS 耐性）を使用しています。
そのため、Boost.Regex / PCRE / JavaScript で使える以下の機能は**非対応**です:

| 機能 | Naire | 代替 |
|---|---|---|
| 先読み `(?=...)` `(?!...)` | ❌ | マッチ部分をキャプチャして書き戻す |
| 後読み `(?<=...)` `(?<!...)` | ❌ | 同上 |
| パターン内後方参照 `(\d)\1` | ❌ | マクロを複数ステップに分割 |
| `\<` `\>` 語境界 | ❌ | `\b` を使用 |
| `\b` `\B` `\A` `\Z` | ✅ | — |
| `(?:...)` 非キャプチャグループ | ✅ | — |
| 最小マッチ `*?` `+?` `??` `{m,n}?` | ✅ | — |

**もっとも頻出するハマりどころ**: `\][^ ]` のように「次の文字が条件付き」の
パターンを書くと、その**次の文字が消費されて失われます**。キャプチャして
復元する形に書き換えてください。

```jsonc
// バグ: ] の直後の文字が消える
{ "kind": "regex", "search": "\\][^ ]", "replace": "] " }

// 修正: キャプチャして復元
{ "kind": "regex", "search": "\\]([^ ])", "replace": "] \\1" }
```

---

## 6. ワイルドカード仕様の詳細

- `?` は任意の 1 文字（任意の Unicode スカラ値）にマッチ
- `*` は 0 文字以上の任意の文字列にマッチ
- 正規表現メタ文字は自動エスケープ — `.zip` はリテラルの `.zip` にマッチ
  （「任意の 1 文字 + zip」ではない）
- パターン全体は `^...$` でアンカーされる — `*.jpg` は末尾が `.jpg` のファイル名のみ一致
- **キャプチャ無し**なので、全マッチは `\0`、ステムは `\t`、拡張子は `\e` で参照

---

## 7. 文字変換構文

`from` と `to` は同じ規則で解析されます。

**列挙モード**（カンマを含む → カンマ区切りのトークン列）:

```json
{ "from": "0,i,ii,iii", "to": "0,1,2,3" }
```

**範囲モード**（カンマなし → 範囲スキャン）:

- `a-z` は 26 個の 1 文字トークンに展開
- `あ-ん` は Unicode ブロック U+3042〜U+3093 のすべてのコードポイントに展開
  （小書きの仮名、濁点ありの仮名なども含む）
- 単独文字は 1 文字トークンとして扱われる。`〇一二三` は 4 トークン
- 文字に挟まれていない `-` はリテラルの `-` として扱われる

長さが異なる場合: `to` が長い場合は余分なトークンを無視、`to` が短い場合は
末尾要素でパディング、`to` が空（`""`）ならマッチ文字を**削除**します。

長いトークンを優先する貪欲マッチなので、`0,i,ii,iii,iv,v,vi,vii,viii,ix,x` の
ような列挙では `viii` が単独のトークンとして解決されます。

---

## 8. Builtin op カタログ

各例は `{ "kind": "builtin", "op": ... }` の `op` 部分の JSON 形式を示します。

### 追加・置換系

#### `add_seq_str` — stem を `{prefix}{seq}{suffix}` で置換

```json
{ "type": "add_seq_str",
  "prefix": "img_", "suffix": "",
  "digits": 3, "start": 0, "step": 1 }
```

ファイル名の stem 部分を構築した文字列で置換し、拡張子はそのまま残します。

#### `add_datetime` — 書式化した日時を先頭または末尾に追加

```json
{ "type": "add_datetime",
  "format": "\\Y\\m\\d_",
  "position": "prefix" }
```

`format` は regex 置換と同じテンプレート構文（§4 参照）。`position` は
`"prefix"`（名前全体の前）または `"suffix"`（stem と拡張子の間）。

#### `add_folder_name` — 親フォルダ名を先頭または末尾に追加

```json
{ "type": "add_folder_name", "position": "prefix" }
```

#### `add_folder_seq` — stem を `{folder_name}{seq}` で置換

```json
{ "type": "add_folder_seq",
  "digits": 3, "start": 0, "step": 1 }
```

#### `truncate_from_start` / `truncate_from_end`

```json
{ "type": "truncate_from_start", "n": 3 }
{ "type": "truncate_from_end",   "n": 4 }
```

**全名**（拡張子含む）の先頭または末尾から `n` 文字を削除します。

### 削除系

#### `delete_chars`

```json
{ "type": "delete_chars",
  "from": "start", "offset": 2, "count": 3 }
```

指定した側から `offset` 文字目の位置から `count` 文字を削除します。`from` は
`"start"` または `"end"`。

#### `delete_before`

```json
{ "type": "delete_before", "from": "start", "n": 3 }
```

`from: "start", n: N` → 先頭から N 文字を削除。
`from: "end",   n: N` → 末尾から N 文字を削除。

#### `delete_pattern`

```json
{ "type": "delete_pattern", "pattern": "copy_num" }
```

`pattern` は以下のいずれか:

| 値 | 削除対象 |
|---|---|
| `copy_num` | ` - Copy`、` - Copy (N)`、` - コピー`、` - コピー (N)`、` コピー (N)` |
| `copy_num_vista` | `ーCopy`、`ーCopy (N)`、`ーコピー`、`ーコピー (N)`（Vista の長音区切り） |
| `shortcut` | ` - Shortcut`、` - ショートカット`、`へのショートカット` |
| `shortcut_vista` | `ーShortcut`、`ーショートカット` |
| `bracket_content` | あらゆる対の括弧とその中身 — `[…]` `(…)` `（…）` `「…」` `『…』` `【…】` `〔…〕` `《…》` |
| `num_kagi` | `【N】` |
| `num_round` | `（N）`（全角丸括弧）|
| `num_lenticular` | `〔N〕` |
| `num_kagi_or_round` | 上 2 つのいずれか |

#### `make_83` — stem を 8 文字に、拡張子を 3 文字に切り詰め（DOS 8.3 形式）

```json
{ "type": "make_83" }
```

### 文字・大文字小文字変換（すべて `skip_ext` 対応）

`skip_ext: true` で変換を stem のみに適用し、拡張子はそのまま残します。

#### `case_convert`

```json
{ "type": "case_convert",
  "conversion": "lower", "skip_ext": true }
```

`conversion`: `"capitalize"`（語頭大文字）/ `"upper"` / `"lower"`。

#### `kana_convert`

```json
{ "type": "kana_convert",
  "conversion": "kata_to_hira", "skip_ext": true }
```

`conversion`:

| 値 | 効果 |
|---|---|
| `hira_to_kata` | ひらがな → カタカナ |
| `kata_to_hira` | カタカナ → ひらがな |
| `hankaku_kata_to_zenkaku` | 半角カナ → 全角カナ（濁点を合成）|
| `zenkaku_kata_to_hankaku` | 全角カナ → 半角カナ（濁点を分解）|

#### `width_convert`

```json
{ "type": "width_convert",
  "conversion": "to_hankaku", "skip_ext": true }
```

`conversion`: `"to_zenkaku"` / `"to_hankaku"` — ASCII 範囲 U+0021〜U+007E ↔
U+FF01〜U+FF5E の英数記号を相互変換。

#### `clear_diacritics`

```json
{ "type": "clear_diacritics", "skip_ext": true }
```

NFD で分解 → すべての結合マーク（ラテン文字のアクセント等）を除去 → NFC で再合成。

#### `remove_voiced_mark` — Naire 独自機能

```json
{ "type": "remove_voiced_mark", "skip_ext": true }
```

NFD 分解後に `U+3099`（結合濁点）と `U+309A`（結合半濁点）を除去して NFC 再合成。
ひらがな・カタカナ両方で動作: `ガ` → `カ`、`ぱ` → `は`。

### 文字列・数値・拡張子ユーティリティ

#### `string_replace` — 全置換のプレーンテキスト

```json
{ "type": "string_replace",
  "search": "foo", "replace": "bar" }
```

正規表現ではなく、厳密な部分文字列マッチ、すべての出現を置換します。

#### `number_pad`

```json
{ "type": "number_pad",
  "from": "start", "n": 1, "digits": 3 }
```

指定した側から `n` 番目の数字列を見つけて `digits` 桁にゼロパディングします。

#### `number_adjust`

```json
{ "type": "number_adjust",
  "from": "end", "n": 1, "delta": -2 }
```

指定した側から `n` 番目の数字列を見つけて `delta` を加算します。元の桁数は
ゼロパディングで維持し、結果は ≥ 0 にクランプされます。

#### `ext_convert`

```json
{ "type": "ext_convert", "conversion": "lower" }
```

`conversion`: `"upper"` / `"lower"`。

#### `ext_delete`

```json
{ "type": "ext_delete" }
```

#### `ext_add`

```json
{ "type": "ext_add", "ext": "bak" }
```

先頭にドットがなければ自動付加されます。

#### `ext_replace`

```json
{ "type": "ext_replace", "ext": "md" }
```

---

## 9. 実例

### 9.1 拡張子一括変更 `.jpg → .png`

正規表現 1 ステップ:

```json
{
  "id": "x", "name": "jpg → png",
  "created_at": "", "updated_at": "",
  "steps": [
    { "kind": "regex", "search": "\\.jpg$", "replace": ".png" }
  ]
}
```

または定型で:

```json
{ "kind": "builtin", "op": { "type": "ext_replace", "ext": "png" } }
```

### 9.2 Windows のコピーマーカー削除

```json
{
  "id": "x", "name": "Windows コピーマーカー除去",
  "created_at": "", "updated_at": "",
  "steps": [
    { "kind": "builtin",
      "op": { "type": "delete_pattern", "pattern": "copy_num" } }
  ]
}
```

### 9.3 作者ソートキー追加（フォルダモード）

`[作者] タイトル` のフォルダ名の先頭に、作者の頭 1 文字を「小文字 / 半角 / 濁点除去」
した形でソートキーとして付与する例:

```json
{
  "id": "x", "name": "作者ソートキー追加",
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

`[ガタタン研究会] 芦別名物` の場合のトレース:

| Step | current |
|---|---|
| 0（初期）| `[ガタタン研究会] 芦別名物` |
| 1 | `ガ` |
| 2 | `が` |
| 3 | `か` |
| 4 | `か`（変化なし）|
| 5 | `か`（変化なし）|
| 6 | `か [ガタタン研究会] 芦別名物` |

### 9.4 フォルダ名 + 3 桁連番でリネーム

```json
{
  "id": "x", "name": "フォルダ名+連番",
  "created_at": "", "updated_at": "",
  "steps": [
    { "kind": "builtin",
      "op": { "type": "add_folder_seq",
              "digits": 3, "start": 1, "step": 1 } }
  ]
}
```

ファイル名は `{folder_name}001.{ext}`、`{folder_name}002.{ext}`、… に置き換わります。

### 9.5 拡張子の前に `_archived` を追加

```json
{
  "id": "x", "name": "archive サフィックス",
  "created_at": "", "updated_at": "",
  "steps": [
    { "kind": "regex",
      "search": "^(.+)(\\.[^.]+)$",
      "replace": "\\1_archived\\2" }
  ]
}
```

拡張子のないファイルにはこの正規表現がマッチしないのでスキップされます
（通常はこれが望ましい挙動）。

---

## 10. ハマりどころ集

1. **先読みの代用**: `(?=...)` ではなく `(...)` で書き、`\1` で復元する。§5 参照。
2. **`.` のエスケープ忘れ**: `\.zip$` はリテラルの `.zip` 拡張子にマッチ、
   `.zip$` は「任意の 1 文字 + zip」にマッチします。
3. **フォルダモード vs ファイルモード**: `\f` はモードに関係なく親フォルダ名を
   指します。フォルダ自身をリネームする場合、`.* → \f` のステップは**不要**で、
   現在の名前に直接操作します。
4. **`alpha` 進数は `digits` を無視**します。bijective base-26（`A`, `B`, …, `Z`,
   `AA`, …, `ZZ`, `AAA`, …）なのでパディングはありません。
5. **`\orig` はマクロ専用**です。単一ステップ実行では `\0` と同じ値ですが、
   そこに依存するのは避け、「現在の名前」を意味するなら `\0` を使ってください。
6. **`replace_all` は全置換**です。1 つ目のマッチだけ置換したい場合は `^` で
   アンカーするか、正規表現をより具体的にしてください。
7. **空の `replace` は許容**で、「マッチした範囲を削除」を意味します。
8. **コンフリクトのあるリネームは実行時に拒否**されます。2 つの行が同じ
   ターゲット名になる場合、ファイルが 1 つも触られる前にバッチ全体が中断します。
9. **チェーンも拒否**されます。同じバッチで `a→b` と `b→c` が両方あるとエラーで
   停止します（仕様上、Naire は安全でないリネーム順序を許しません）。
   2 回に分割するか、正規表現を見直してください。

---

## 11. AI ライター向けプロンプトテンプレート

LLM に Naire マクロを書いてもらう場合、このドキュメント全体を貼り付けた上で
以下のような指示を出してください:

```
以上の Naire マクロリファレンスを読んだ上で、次のタスクのための
Macro JSON オブジェクト（配列形式ではなくオブジェクト形式）を 1 つ生成してください:

  <タスクの内容を自然言語で記述>

ルール:
- id は短い任意のプレースホルダで OK（インポート時に再生成されます）
- created_at / updated_at は ISO 8601 文字列
- 意図に合う定型があれば、正規表現より定型 op を優先
- Rust regex に先読みはないので、キャプチャ&復元方式を使う
- リテラルとして扱いたい正規表現メタ文字はエスケープ
- ワイルドカードはキャプチャが無いので、stem / 拡張子は \t / \e で参照
- 使わないステップ種別は省略し、実際に使うものだけ含める

JSON のみ、コードブロックで囲み、説明文は付けないでください。
```

生成された JSON を `.json` ファイルとして保存し、Naire のマクロパネルで
**↑ インポート** から読み込めば使えます。
