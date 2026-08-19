# Feature Plan: Diff Checker

**Tool id:** `diff_checker`
**Tool name:** Diff Checker
**Status:** planned
**Framework:** see `docs/tool-framework.md`

## 1. Overview

Visualize the difference between two input texts. The GUI supports two views:

1. **Side-by-side view** with aligned lines and single-word diff highlighting.
2. **Track-changes view** in the style of Microsoft Word: deleted text is red with strikethrough, inserted text is green with underline.

The CLI exposes the same diff engine and can render side-by-side text, track-changes HTML, or unified diff output.

## 2. Scope

### In scope
- Line-level diff with word-level refinement inside changed lines.
- Side-by-side structured output.
- Word-style track-changes structured output.
- CLI side-by-side (ANSI/plain), track-changes (HTML), and unified formats.
- GUI input from textareas, paste, and file open.
- Empty vs non-empty input diffs.

### Out of scope (v1)
- Rich-text inputs (docx/rtf) — v1 compares plain text only.
- Three-way merge.
- Diff of PDFs or images.
- Interactive editing/merge in the GUI.
- Extremely large files (>50 MB) get line-level only with a notice (see 6).

## 3. Core Design (`doctk-core/src/features/diff_checker.rs`)

### 3.1 Data model

```rust
pub enum LineKind { Equal, Delete, Insert, Replace }

pub struct WordRange {
    pub left_start: usize,
    pub left_len: usize,
    pub right_start: usize,
    pub right_len: usize,
}

pub struct SideBySideLine {
    pub line_number_left: Option<usize>,
    pub line_number_right: Option<usize>,
    pub left_text: String,
    pub right_text: String,
    pub kind: LineKind,
    pub word_diff: Vec<WordRange>,
}

pub struct SideBySideDiff {
    pub lines: Vec<SideBySideLine>,
}

pub enum TrackChangeKind { Equal, Inserted, Deleted }

pub struct TrackChangeSegment {
    pub text: String,
    pub kind: TrackChangeKind,
}

pub struct TrackChangesDiff {
    pub segments: Vec<TrackChangeSegment>,
}
```

All models derive `Serialize`/`Deserialize` for CLI JSON and Tauri transport.

### 3.2 Public API

```rust
pub const MANIFEST: ToolManifest;

pub fn diff_side_by_side(left: &str, right: &str) -> SideBySideDiff;
pub fn diff_track_changes(left: &str, right: &str) -> TrackChangesDiff;
```

### 3.3 Algorithm

1. **Normalize** line endings to `\n`.
2. **Line-level diff** with `similar::TextDiff` (Myers default; patience available behind an option later).
3. **Emit lines**:
   - Equal line → `LineKind::Equal`, both sides filled, no word diff.
   - Delete → left text only, right blank.
   - Insert → right text only, left blank.
   - Replace → both sides filled and word diff computed.
4. **Word-level diff** for replace lines:
   - Tokenize each line with a custom tokenizer:
     - Whitespace is attached to the preceding token so reconstruction is lossless.
     - CJK characters are treated as individual tokens (so Chinese/Japanese/Korean show single-character highlights).
     - Punctuation is its own token.
   - Run token-level diff with `similar`.
   - Map token-level operations back to UTF-8 byte ranges for `WordRange`.
5. **Track changes build**:
   - Equal line → `Equal` segment including newline.
   - Deleted line → `Deleted` segment including newline.
   - Inserted line → `Inserted` segment including newline.
   - Replace line → word-level segments (`Deleted`/`Inserted`/`Equal`), with only changed words marked; newline attached to the last segment.

### 3.4 Invariants

- Applying all `Inserted` segments and removing all `Deleted` segments from `left` must reconstruct `right` (track-changes round-trip invariant).
- `WordRange` values are UTF-8 byte offsets into the corresponding line string.
- Output is deterministic for identical inputs.

## 4. CLI Design (`doctk-cli/src/commands/diff_checker.rs`)

```bash
doctk diff LEFT RIGHT [--format side-by-side|track-changes|unified] [-o OUTPUT] [--exit-code]
```

- `LEFT`, `RIGHT` are file paths; `-` means stdin.
- `--format`:
  - `side-by-side` (default): terminal text; ANSI colors if stdout is a TTY, plain text if piped.
  - `track-changes`: standalone HTML document with Word-like styles.
  - `unified`: standard unified diff with `@@` hunks.
- `--exit-code`: exit `1` when differences exist, `0` when identical (like GNU diff).
- `-o OUTPUT`: write to file instead of stdout.

### CLI rendering details

- Side-by-side ANSI: deleted lines prefixed `-` and red; inserted lines prefixed `+` and green; replaced words inverse/highlight.
- Track-changes HTML: `<del style="color:red;text-decoration:line-through">` and `<ins style="color:green;text-decoration:underline">`.

## 5. GUI Design (`gui/src/features/diff-checker/`)

### 5.1 Layout

```text
[ Original                          ]            [ Changed                           ]
[ (Open file…) small, left-aligned  ]  [ Swap ]  [ (Open file…) small, right-aligned ]
[ textarea (scrollable)             ]            [ textarea (scrollable)             ]
------------------------------------------------------------------------------------
[ toolbar: (•) Side-by-side  ( ) Track changes          Copy: [Changed ▾] (right) ]
------------------------------------------------------------------------------------
[ Diff output pane                                                                    ]
[ Side-by-side: aligned two-column view with word highlights                           ]
[ Track changes: single document with red strikethrough / green underline              ]
```

- The left box is titled **Original**; the right box is titled **Changed**.
- Each box has a small **Open file…** button above its textarea. The Original
  box button is left-aligned; the Changed box button is right-aligned.
- The **Swap** button is rendered in its own **middle column** between the two
  boxes, vertically aligned with the **Open file…** buttons row. It uses a
  two-arrow icon (`⇄`) with an accessible label. It is not placed next to the
  view mode buttons.
- The view mode buttons (**Side by side** / **Track changes**) are on the left
  of a toolbar below the input boxes.
- When the **Track changes** view is active, the same toolbar shows a
  right-aligned **Copy:** dropdown with options **Raw**, **Original**, and
  **Changed**. Default is **Changed**.

### 5.2 Behavior

- Two textareas hold the **Original** (left) and **Changed** (right) inputs.
- File open uses the Tauri dialog plugin; each box has its own small **Open
  file…** button and content loads into the corresponding textarea.
- Diff recomputes on input change (debounced ~250 ms) or on explicit **Run diff** button.
- **Swap** exchanges the Original and Changed inputs. The button sits in the
  middle column between the two Open file buttons, not in the view-mode toolbar.
- View toggle switches between side-by-side and track-changes; both views render from the same `SideBySideDiff`/`TrackChangesDiff` result. The view toggle toolbar is below the input boxes and left-aligned.
- Side-by-side view:
  - Aligned rows with red/green line backgrounds and `-`/`+` gutters.
  - Replaced lines render both versions with word-level `<mark>`-like highlights.
  - Left and right columns share a synchronized scroll position.
- Track-changes view:
  - Renders structured segments as React elements: deleted = red strikethrough, inserted = green underline.
  - `Ctrl+C` (Windows/Linux) and `Cmd+C` (macOS) respect the **Copy:** dropdown mode:
    - `Raw`: browser default selected visible text.
    - `Original`: selected text mapped back to the original side.
    - `Changed`: selected text mapped back to the changed side.
  - No raw HTML injection; all rendering is component-based.
- Side-by-side word highlights:
  - Core returns word ranges as **UTF-8 byte offsets**. The frontend converts
    these byte offsets to JavaScript string indices before slicing, so
    highlighted words stay aligned with non-ASCII text such as curly quotes or
    CJK characters.
- Empty inputs are valid; a diff against empty shows all lines as inserted/deleted.

### 5.3 Tauri commands

```rust
#[tauri::command] pub async fn diff_checker_side_by_side(left: String, right: String) -> Result<SideBySideDiff, String>;
#[tauri::command] pub async fn diff_checker_track_changes(left: String, right: String) -> Result<TrackChangesDiff, String>;
```

## 6. Performance & Limits

- For inputs > 50 MB, core falls back to line-level diff only and returns a flag (or the GUI shows a notice) to avoid quadratic word-diff cost.
- Debounce in GUI prevents recompute on every keystroke.
- Core is CPU-bound; Tauri commands run in `spawn_blocking`.

## 7. Edge Cases

| Case | Expected behavior |
|---|---|
| Identical inputs | All lines equal; track changes all `Equal` |
| Empty left | All right lines `Insert`; track changes all `Inserted` |
| Empty right | All left lines `Delete`; track changes all `Deleted` |
| One word changed in a long line | Only that word highlighted; rest of line `Equal` |
| CRLF vs LF | Treated as equal after normalization |
| CJK text | Character-level word diff |
| Punctuation-only change | Punctuation token highlighted |
| Very long line (no newline) | Word diff still computed; line rendered with wrapping in GUI |
| 10k+ lines | UI stays responsive; async compute + debounce |

## 8. Testing

### Core (unit + fixtures)
- Identical texts.
- Single insert/delete line.
- Replace line with single word change.
- UTF-8 and CJK word ranges.
- CRLF normalization.
- Track-changes round-trip invariant (apply insertions/deletions to left → right).
- Determinism for large generated inputs.

### CLI (`assert_cmd`)
- Side-by-side plain output for fixture pair.
- Track-changes HTML output contains `<del>`/`<ins>`.
- Unified diff output.
- `--exit-code` behavior.
- stdin input via `-`.

### GUI (Vitest)
- Diff view rendering helpers (segment → React props).
- Debounce hook.
- Side-by-side scroll sync logic.

## 9. Future Extensions

- Inline vs side-by-side word diff toggles.
- Ignore whitespace/case options.
- Compare docx/rtf by extracting text.
- Export track changes as `.docx`.
- Character-level diff for all scripts.
