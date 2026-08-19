# Feature Plan: Markdown → Text

**Tool id:** `markdown_text`
**Tool name:** Markdown → Text
**Status:** planned
**Framework:** see `docs/tool-framework.md`

## 1. Overview

Convert markdown source to plain text by removing common markdown syntax. For
example, `# Header` becomes `Header`. Links are handled specially:

- URL links keep the target in parentheses: `[text](https://example.com)` → `text (https://example.com)`
- Section links within the same markdown file are removed: `[text](#section)` → `text`

The feature is available in the GUI and the CLI and follows the standard
four-layer feature contract (core, CLI, Tauri, frontend).

## 2. Scope

### In scope (v1)

- ATX headings (`#` … `######`), with optional closing `#` markers.
- Emphasis / strong / bold-italic:
  - `*italic*`, `_italic_`
  - `**bold**`, `__bold__`
  - `***bold italic***`, `___bold italic___`
- Strikethrough: `~~text~~` → `text`.
- Inline code spans: `` `code` `` → `code`.
- Fenced code blocks:
  - ```` ``` ```` and `~~~` fences are removed.
  - Code content is preserved verbatim and is not processed for inline
    markdown.
- Blockquotes: leading `>` plus one optional space is removed per line.
- Unordered list markers: `-`, `*`, `+` at line start are removed.
- Ordered list markers: `1.`, `2)` etc. at line start are removed.
- Horizontal rules (`---`, `***`, `___` on their own line) are removed.
- Links:
  - URL targets are kept in parentheses exactly as written: `text (url)`.
  - Internal section targets (`#section`, `#section-2`) are removed: `text`.
  - Other non-empty targets (e.g. `other-file.md`) are also kept in
    parentheses, exactly as written.
- Images: `![alt](target)` → `alt` (the image syntax is removed and only the
  alt text is kept).
- Autolinks: `<https://example.com>` → `https://example.com`.

### Out of scope (v1)

- Reference-style links (`[text][id]`, `[id]: url`) and footnotes.
- Link targets containing nested unescaped parentheses.
- Multi-line inline constructs (e.g. emphasis spanning multiple paragraphs).
- HTML tags are left as-is (not stripped, not interpreted).
- Markdown tables are left unchanged as pipe text.
- Setext headings (`Header\n======`) are not supported and are not converted
  to headings. Only `---`, `***`, and `___` on their own line are removed as
  horizontal rules.
- YAML front matter is not specially handled.

## 3. Core Design (`doctk-core/src/features/markdown_text.rs`)

### 3.1 Public API

```rust
pub const MANIFEST: ToolManifest;

/// Convert markdown to plain text. The conversion is best-effort and does not
/// fail; malformed markdown is emitted as-is or after light normalization.
pub fn markdown_to_text(md: &str) -> String;
```

Note: unlike parsing-oriented features, markdown-to-text is intentionally
infallible. The CLI and Tauri command wrap the returned string in `Ok`.

### 3.2 Algorithm

1. Normalize line endings to `\n`.
2. Extract fenced code blocks from the input and replace them with internal
   placeholders so their content is protected from inline processing. Fence
   lines are removed; code content is preserved verbatim.
3. Process each remaining line:
   - Skip horizontal-rule-only lines (`---`, `***`, `___`, with optional
     whitespace).
   - Remove ATX heading markers (`^#{1,6}\s+`) and optional trailing `#`
     markers; trim the heading text.
   - Remove blockquote markers (`^>\s?`).
   - Remove list markers at the start of the line:
     - unordered: `^\s*[-+*]\s+`
     - ordered: `^\s*\d+[.)]\s+`
4. Apply inline conversions in the following order:
   - Inline code spans: `` `code` `` → `code`.
   - Images: `![alt](target)` → `alt`.
   - Inline links: `[text](target)` → `text` or `text (target)` per link rules.
   - Autolinks: `<http://...>` / `<https://...>` / `<mailto:...>` → inner URL.
   - Strikethrough: `~~text~~` → `text`.
   - Emphasis / strong / bold-italic markers removed.
5. Restore fenced code block content at the original positions.
6. Trim trailing whitespace per line.
7. Collapse 3 or more consecutive blank lines into 2 blank lines.

### 3.3 Link rules

| Link target | Output |
|---|---|
| Starts with `#` (e.g. `#section`, `#`, `#section-2`) | `text` |
| URL-like: `http://`, `https://`, `ftp://`, `mailto:`, `//`, or starts with `www.` | `text (target)` |
| Any other non-empty target (e.g. `other-file.md`, `./doc.md#part`) | `text (target)` |
| Empty target `[]()` or `[text]()` | `text` |

### 3.4 Examples

```text
# Header            -> Header
**bold** text       -> bold text
~~gone~~            -> gone
`code`              -> code
- item one          -> item one
1. first item       -> first item
> quoted            -> quoted
[site](https://a.io) -> site (https://a.io)
[section](#part)    -> section
![alt](img.png)     -> alt
<https://a.io>      -> https://a.io
```

## 4. CLI Design (`doctk-cli/src/commands/markdown_text.rs`)

```bash
doctk md2text [INPUT] [-o OUTPUT]
```

- `INPUT` defaults to stdin (`-`); `OUTPUT` defaults to stdout (`-`).
- UTF-8 text only.
- Exit code `0` on success.
- Register the command in `commands/mod.rs` (`subcommands()` and `dispatch()`).

## 5. GUI Design (`gui/src/features/markdown-text/`)

### 5.1 Layout

```text
[ Markdown (plain text) — full width, default height H                    ]
[ [ textarea (monospace, scrollable)                                    ] ]
[ toolbar (left-aligned): [Convert to Text ↓]                             ]
[ Plain text output — full width, default height H                        ]
[ [ textarea (read-only, monospace, scrollable, copyable)               ] ]
[ toolbar (left-aligned): [Copy output]                                   ]
```

- Vertical layout consistent with the Markdown ⇄ TSV tool.
- The output box is the same default height `H` as the input box.
- The output textarea is read-only and selectable so users can copy the plain
  text directly.

### 5.2 Behavior

- The user edits markdown in the input box and clicks **Convert to Text ↓**.
- The conversion runs in the Tauri backend via the core function; the result is
  placed in the output textarea.
- **Copy output** writes the full plain-text result to the clipboard using
  `navigator.clipboard.writeText`.
- Conversion is button-driven, not automatic.
- Switching to another tool tab and back preserves the input and output text
  (the component is kept mounted).

### 5.3 Tauri command

```rust
#[tauri::command]
pub fn markdown_text_convert(md: String) -> Result<String, String>;
```

- Command name is prefixed by the tool id.
- The command simply calls `markdown_to_text(&md)` and returns `Ok(text)`.

## 6. Registration Checklist

Per `docs/tool-framework.md`, add the feature in all layers:

- [ ] Core: `crates/doctk-core/src/features/markdown_text.rs`
  - `pub const MANIFEST`
  - `pub fn markdown_to_text(&str) -> String`
  - unit tests
  - register in `features/mod.rs` and `registry.rs`
- [ ] CLI: `crates/doctk-cli/src/commands/markdown_text.rs`
  - `cli()` and `run()`
  - register in `commands/mod.rs`
  - integration tests with `assert_cmd`
- [ ] Tauri: `gui/src-tauri/src/features/markdown_text.rs`
  - `#[tauri::command] pub fn markdown_text_convert(...)`
  - register in `features/mod.rs`
- [ ] Frontend: `gui/src/features/markdown-text/index.tsx`
  - register in `gui/src/toolRegistry.tsx`
  - add `markdownText` invoke wrapper in `gui/src/lib/tauri.ts`
- [ ] Docs: update `PLAN.md` index and `README.md` feature table

## 7. Testing

### Core (unit + fixtures)

- Headings `#` through `######`.
- Bold, italic, bold-italic.
- Strikethrough.
- Inline code spans.
- Fenced code blocks (``` and ~~~) preserve content and do not process inline
  syntax inside.
- Blockquotes.
- Unordered and ordered list markers.
- Horizontal rules.
- URL links are kept in parentheses.
- Internal section links are removed.
- Images keep alt text only.
- Autolinks.
- CRLF normalization.
- Mixed document smoke test.

### CLI (`assert_cmd`)

- File input and stdin input.
- Output file with `-o`.
- `# Header` → `Header` through the CLI.

### GUI (Vitest / manual)

- `markdownToText` invoke wrapper (type/build-level).
- Manual test: input `# Header` → output `Header`.
- Manual test: `[site](https://a.io)` → `site (https://a.io)`.
- Manual test: `[section](#part)` → `section`.

## 8. Future Extensions

- Reference-style links and footnotes.
- Setext headings.
- Markdown table to plain-text rows.
- HTML tag stripping.
- YAML front matter removal.
- Options for link target handling (keep/remove for all non-URL links).
- Live conversion on debounced input.
