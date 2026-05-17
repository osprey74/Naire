# Naire

> A modern bulk file/folder rename tool for Windows and macOS — the spiritual successor to *Flexible Renamer* (last updated 2011).

[日本語版 README はこちら](./README.ja.md)

Naire is a cross-platform desktop application that brings powerful, predictable, and Japanese-aware bulk renaming to modern operating systems. It supports regular expressions, wildcards, 22 built-in templates, scriptable macros, sequence numbering, and — new in v1.0.0 — **folder grouping** that consolidates multiple sibling folders under a common parent in a single operation.

Built with Tauri v2 + React + TypeScript + Rust.

---

## Features

### Renaming engines

- **Regex mode** — Rust `regex` crate (linear-time, ReDoS-safe), capture groups, rich replacement variables
- **Wildcard mode** — `?` `*` translated to anchored regex
- **Character convert mode** — `tr`-style 1:1 mapping with range (`a-z`) and enumeration (`0,i,ii,iii,...`) syntax
- **22 built-in templates** covering case / kana / width / diacritics / voiced-mark removal / sequence / date / folder name / delete patterns / 8.3 truncation / number padding & adjustment / extension manipulation
- **Macro system** — sequence steps in any order; step-by-step execution with rewind; `\orig` variable references the macro-start name
- **Macro JSON import / export** — share macros as `.json`, including Claude-generated ones

### Folder grouping (new in v1.0.0)

- Select multiple sibling folders → Naire extracts the longest common prefix and creates a new parent folder named after it
- Selected folders are moved into the new parent in a single operation
- Optional sequence-numbering rename applies inside the new parent (e.g. `Vol01`, `Vol02`, ... → `01`, `02`, ...)
- Full UNDO support: the grouping operation (create directory + move + rename) is one stack entry, fully reversible with `Ctrl+Z`

### Japanese filename support

- Hiragana ↔ katakana conversion (`U+3041–U+3096` ↔ `U+30A1–U+30F6`)
- Full-width ↔ half-width conversion (alnum + katakana)
- **Voiced / half-voiced mark removal** via NFD normalization (e.g. `が` → `か`, `ぱ` → `は`, `ガ` → `カ`)
- Diacritical mark removal for Latin scripts

### Replacement variables

| Variable | Meaning |
|---|---|
| `\0` | current name (full filename) |
| `\t` | current title (without extension) |
| `\e` | current extension (with leading dot) |
| `\f` | parent folder name |
| `\F` | grandparent folder name |
| `\;` / `\:` | file size (with / without comma separators) |
| `\Y` `\y` `\m` `\d` `\H` `\I` `\M` `\S` | date components from file mtime |
| `\a` `\A` `\b` `\B` `\p` | locale-aware weekday / month / AM-PM |
| `\#X` | strip leading zero from the next date variable |
| `\1`–`\9` | regex capture groups |
| `\u` `\l` `\U` `\L` `\E` | case-control modifiers |
| `?` `??` `???` `????` | 1–4 digit sequence number |
| `\orig` | original filename at macro step 0 (macros only) |
| `\?` `\\` | literal `?` / `\` |

### Workspace features

- **Folder browser** — explorer-style tree (left pane), macOS `/Volumes/*`, Windows drive letters, mapped network drives
- **Display filter** — semicolon-OR glob patterns (`*.jpg;*.png`), negation (`^(*.tmp)`), 7 built-in presets, 10-entry history
- **Live preview** — see the new name beside the original; changed rows highlighted
- **Row selection** — click / Ctrl+click (toggle) / Shift+click (range); rename only the selected rows when a subset is selected
- **Resizable columns** with persistent widths
- **Sub-folder recursion** with depth control (0 = unlimited)
- **Sequence counter** — decimal / hex / alpha (bijective base-26), per-folder reset option
- **UNDO** — last 20 batches, `Ctrl+Z` keyboard shortcut
- **Support button** — context-aware dropdown to insert variables, character classes, and presets
- **Settings persistence** — folder, mode, filter, filter history, sequence config, column widths, macro library, window state

### Cross-platform

| | Windows | macOS |
| --- | --- | --- |
| OS | Windows 10 or later | macOS 10.15 (Catalina) or later |
| Architecture | x86_64 | x86_64, Apple Silicon (ARM) |

---

## Installation

Download the installer for your platform from the [Releases page](https://github.com/osprey74/Naire/releases/latest).

- **Windows**: `Naire_<version>_x64-setup.exe` (MSI installer) or `Naire_<version>_x64_en-US.msi`
- **macOS**: `Naire_<version>_universal.dmg` (universal binary, runs natively on Intel and Apple Silicon)

After install, launch **Naire** from the Start Menu / Applications folder.

---

## Quick start

1. Click on a folder in the left-pane folder tree to choose a working folder.
2. Use the **Target** toggle in the toolbar to pick *Files* or *Folders*.
3. Optionally enable **Recursive** for sub-folder traversal.
4. Choose a rename mode in the center panel: **定型 (Built-in)** / **高度な (Advanced)** / **マクロ (Macro)** / **集約 (Group)**.
5. Configure the rename parameters; the right pane updates in real time.
6. Select a subset of rows if needed (Ctrl+click / Shift+click).
7. Click **リネーム実行 (Rename)** at the bottom, or **集約実行 (Group)** when in Group mode.
8. `Ctrl+Z` undoes the last batch (up to 20 levels).

For a deeper macro authoring guide, see [docs/MACRO_REFERENCE.md](./docs/MACRO_REFERENCE.md) (English) or [docs/MACRO_REFERENCE.ja.md](./docs/MACRO_REFERENCE.ja.md) (Japanese).

---

## Building from source

### Prerequisites

- Node.js 20+ (or Bun)
- Rust stable (`rustup`)
- Platform-specific Tauri prerequisites — see [tauri.app prerequisites](https://tauri.app/start/prerequisites/)

### Steps

```bash
# Clone
git clone https://github.com/osprey74/Naire.git
cd Naire

# Install JS dependencies
npm install

# Dev mode (hot reload)
npm run tauri dev

# Production build (creates installer in src-tauri/target/release/bundle/)
npm run tauri build
```

Backend tests:

```bash
cd src-tauri
cargo test --lib
```

As of v1.0.0, 127 tests cover the rename engine, macro runner, folder grouping, undo, and file system operations.

---

## Architecture

```
src/                       # React + TypeScript frontend (Vite)
├── App.tsx                # main shell, state orchestration
├── components/            # UI components (Toolbar, FolderTree, ModePanel, PreviewPanel, ...)
└── types/rename.ts        # type definitions shared with Rust DTOs

src-tauri/                 # Rust backend (Tauri v2)
├── src/commands.rs        # Tauri command handlers (DTOs + I/O orchestration)
├── src/filter.rs          # display filter (globset)
├── src/macro_io.rs        # macro JSON import/export
└── src/rename/
    ├── advanced.rs        # regex + wildcard
    ├── builtin.rs         # 22 built-in operations
    ├── char_convert.rs    # tr-style character mapping
    ├── convert.rs         # kana / width / diacritics / voiced-mark
    ├── group.rs           # folder grouping (Phase 14)
    ├── macro_runner.rs    # macro step processor
    ├── sequence.rs        # decimal / hex / alpha sequence formatter
    ├── undo.rs            # batch undo (Rename / CreateDir ops)
    └── variables.rs       # replacement variable expansion
```

The architectural principle: **all rename logic lives in Rust**. The frontend never performs string manipulation on filenames — it sends parameters to the backend and renders the returned preview.

---

## Releases & versioning

Naire follows [Semantic Versioning](https://semver.org/).

- **v1.0.0** (2026-05-17) — General Availability. Adds folder grouping (Phase 14), formalizes Naire as a stable product.
- **v0.2.0** (2026-05-13) — UX polish: window state persistence, locale-aware date variables, case-control variables, drag-and-drop macro steps, network-mounted volumes on macOS.
- **v0.1.0** (2026-05-12) — Initial public release.

See [`RELEASE_NOTES_v1.0.0.md`](./RELEASE_NOTES_v1.0.0.md) for the latest detailed notes.

---

## License

This project is provided as personal-use software by the author. See repository for terms.

Icon attribution: Several icons used in this application were obtained from [Flaticon](https://www.flaticon.com/). The「アプリについて」(About) dialog in the application contains the full attribution required by Flaticon's license.

---

## Acknowledgements

- *Flexible Renamer* (Naotaka Hirayama, 2000–2011) — the original Windows bulk rename tool that defined the feature set and UX vocabulary Naire builds on.
- [Tauri](https://tauri.app/), [React](https://react.dev/), [Rust regex](https://docs.rs/regex/), [`@dnd-kit/sortable`](https://dndkit.com/), and the wider Rust + TypeScript open-source ecosystems.

---

## Contributing

Issues and feature requests are welcome at [github.com/osprey74/Naire/issues](https://github.com/osprey74/Naire/issues).
