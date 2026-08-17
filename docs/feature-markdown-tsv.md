# Feature Plan: Markdown Table ↔ TSV Conversion

**Tool id:** `markdown_tsv`
**Tool name:** Markdown ⇄ TSV
**Status:** planned
**Framework:** see `docs/tool-framework.md`

## 1. Overview

Convert between GitHub-style markdown tables and plain TSV (tab-separated values). In the GUI, markdown is edited as plain text in a textarea, while TSV is shown as an editable table viewer that supports editing cells, deleting/inserting rows and columns, and converting back to markdown.

## 2. Scope

### In scope
- Parse and generate GitHub-style markdown tables (with or without leading/trailing pipes).
- Preserve and emit column alignment (`---`, `:---`, `:---:`, `---:`).
- Parse and generate plain TSV (no quoting/escaping).
- Editable GUI grid with cell edit, insert/delete rows, insert/delete columns, undo/redo.
- Two-way GUI conversion (markdown → table, table → markdown).
- CLI conversion commands with file or stdin/stdout I/O.

### Out of scope (v1)
- Multi-line cells in TSV (cells containing tabs or newlines).
- Markdown tables nested inside blockquotes/lists.
- Pipes inside inline code spans or links are not specially parsed; only `\|` escaping is supported.
- CSV, XLSX, or other spreadsheet formats.

## 3. Core Design (`doctk-core/src/features/markdown_tsv.rs`)

### 3.1 Data model

```rust
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub struct MarkdownTable {
    pub table: Table,
    pub alignments: Vec<Alignment>,
}

pub enum Alignment { Default, Left, Center, Right }

pub struct TableError {
    pub line: Option<usize>,
    pub message: String,
}
```

### 3.2 Public API

```rust
pub const MANIFEST: ToolManifest;

pub fn parse_markdown_table(text: &str) -> Result<MarkdownTable, TableError>;
pub fn markdown_table_to_string(md: &MarkdownTable) -> String;
pub fn parse_tsv(text: &str) -> Result<Table, TableError>;
pub fn tsv_to_string(table: &Table) -> String;

// Convenience round-trip helpers used by CLI and GUI:
pub fn markdown_to_tsv(md_text: &str) -> Result<String, TableError>;
pub fn tsv_to_markdown(tsv_text: &str) -> Result<String, TableError>;
```

### 3.3 Markdown parsing rules

1. Normalize line endings to `\n`; trim blank lines around the table.
2. First non-empty line = header row.
3. Second line = delimiter row. Each delimiter cell must match `:?-{3,}:?` after trimming.
4. Remaining non-empty lines = body rows.
5. Cell splitting:
   - Split each row on unescaped `|`.
   - Remove one optional leading and one optional trailing pipe.
   - Trim whitespace around each cell.
   - Unescape `\|` to `|`.
6. Column count:
   - Header count `N` defines the table width.
   - Every body row and delimiter row must have exactly `N` columns; otherwise return `TableError { line: Some(line_no), message: "expected N columns, found M" }`.
7. Alignment:
   - `:---` → Left
   - `---:` → Right
   - `:---:` → Center
   - `---` → Default
8. Single-column tables without pipes are accepted (the whole line is one cell).

### 3.4 Markdown generation rules

- Escape `|` as `\|` in every cell.
- Replace newlines in cells with `<br>`; replace tabs with spaces.
- Emit `| h1 | h2 |` header, alignment delimiter row, then body rows.
- Emit compact form (no padding); GUI may optionally align columns for readability.
- Preserve alignment from the parsed table; when converting from a plain grid with no alignment info, emit `---` for every column.

### 3.5 TSV parsing rules

- Rows split by `\n`; `\r\n` normalized to `\n`.
- Cells split by `\t`.
- No quoting/escaping semantics.
- A trailing newline does not create an extra empty row.
- Empty input → `Table { headers: [], rows: [] }`.
- Inconsistent column counts → `TableError` with line number.
- Because TSV cannot represent tabs/newlines inside a cell, markdown-to-TSV conversion replaces them as above.

## 4. CLI Design (`doctk-cli/src/commands/markdown_tsv.rs`)

```bash
doctk table md2tsv [INPUT] [-o OUTPUT]
doctk table tsv2md [INPUT] [-o OUTPUT]
```

- `INPUT` defaults to stdin (`-`); `OUTPUT` defaults to stdout (`-`).
- UTF-8 text only.
- On parse error: print `error: <message>` to stderr, exit code `1`.
- `clap::Command` name is `table`, with subcommands `md2tsv` and `tsv2md`.

## 5. GUI Design (`gui/src/features/markdown-tsv/`)

### 5.1 Layout

```text
[ Markdown (plain text) ]                [ TSV (editable table)         ]
[ textarea                ]  --Convert--> [ editable grid               ]
[                         ]  <--Convert-- [ toolbar: +row -row +col -col]
[ status bar: line/col    ]               [ status bar: R rows × C cols ]
```

### 5.2 Behavior

- **Markdown → table**: user edits markdown and clicks **Convert to Table**. On success the grid is replaced with the parsed table; on error an inline message appears and the previous grid stays intact.
- **Table → markdown**: user edits the grid and clicks **Convert to Markdown**. The markdown textarea is updated. A dirty indicator marks whether grid edits have not yet been converted.
- Conversion is button-driven (not automatic) to prevent one pane from clobbering the other while the user types.
- Import/Export: open/save TSV files via Tauri dialog plugin; open markdown file into the textarea; save markdown textarea.

### 5.3 Editable grid (`EditableTsvGrid.tsx`)

- Renders `headers` and `rows` from `Table`.
- Cell editing:
  - Double-click, Enter, or F2 to edit.
  - Enter commits and moves down; Tab commits and moves right; Escape cancels.
  - Arrow keys navigate without editing.
- Selection:
  - Click cell to select; click row/column header to select whole row/column.
  - Shift+click for a range; toolbar actions apply to the current selection.
- Row operations: insert above/below, delete selected row(s), delete row via row-header context menu.
- Column operations: insert left/right, delete selected column(s), delete column via column-header context menu.
- Undo/redo stack for cell edits and row/col operations.
- Cell sanitization on commit: tabs and newlines are replaced with spaces (keeps the TSV model valid).
- Grid state is a `string[][]` in React; conversion to/from core only happens at the feature boundary.
- Grid operations (`insertRow`, `deleteRows`, `insertCol`, `deleteCols`) are pure functions in `gui/src/features/markdown-tsv/gridOps.ts` with Vitest tests.

### 5.4 Tauri commands

```rust
#[tauri::command] pub async fn markdown_tsv_md_to_table(md: String) -> Result<Table, String>;
#[tauri::command] pub async fn markdown_tsv_table_to_md(table: Table) -> Result<String, String>;
```

- Command names are prefixed by tool id per framework convention.
- The frontend works with the `Table` JSON structure directly; markdown/TSV strings are only generated on export or conversion.

## 6. Edge Cases

| Case | Expected behavior |
|---|---|
| Empty markdown | Error `no table found` in CLI; GUI shows inline message |
| Malformed delimiter row | Error with line number |
| Row with extra/missing columns | Error with line number and expected/actual counts |
| Escaped pipe `\|` inside cell | Parsed as literal `|`; re-escaped on output |
| Single-column table with no pipes | Parsed and regenerated |
| CRLF TSV | Normalized and converted |
| Empty TSV file | Empty table; markdown generation produces empty string |
| Cell contains tab/newline | Replaced with space on grid commit; `<br>`/space on markdown-to-TSV |
| Alignment colons | Preserved on markdown round-trip |

## 7. Testing

### Core (unit + fixtures)
- Round-trip `markdown -> TSV -> markdown`.
- With/without leading-trailing pipes.
- Escaped pipes.
- All four alignment styles.
- Single-column table.
- Malformed delimiter row.
- Inconsistent columns (line number reported).
- CRLF, trailing newline, empty input.

### CLI (`assert_cmd`)
- `md2tsv` file input → stdout matches fixture.
- `tsv2md` file input → stdout matches fixture.
- stdin and `-o` output file paths.
- Parse error exits with code 1 and stderr message.

### GUI (Vitest)
- `gridOps.ts` insert/delete row/col with selection ranges.
- Undo/redo stack behavior.
- Cell sanitization.

## 8. Future Extensions

- CSV and XLSX import/export.
- Auto-convert on debounce.
- Spreadsheet paste from Excel/Google Sheets.
- Sorting and filtering.
- Column width drag and cell formatting.
