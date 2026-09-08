# Feature Plan: Case Converter

**Tool id:** `case_converter`
**Tool name:** Case Converter
**Status:** planned
**Framework:** see `docs/tool-framework.md`

## 1. Overview

Convert text between five common cases:

1. **Sentence case** — lowercase everything, then capitalize the first letter of
   each sentence and any user-specified proper noun.
2. **lower case** — convert all letters to lowercase.
3. **UPPER CASE** — convert all letters to uppercase.
4. **Capitalized Case** — capitalize the first letter of every word.
5. **Title Case** — capitalize the first and last word of each title, major
   words, and long words, while keeping minor words (such as `a`, `an`, `the`,
   `and`, `of`) lowercase in the middle; apply user-specified proper nouns.

The feature follows the standard four-layer contract (core, CLI, Tauri,
frontend). In the GUI sidebar it appears **immediately after Diff Checker**.
The GUI gives each mode its own direct conversion button; there is no mode
dropdown, no separate **Convert** section, and no mode hint line.

## 2. Scope

### In scope (v1)

- All five conversion modes above.
- User-specified proper nouns for Sentence case and Title Case (GUI text area,
  one noun or phrase per line; CLI repeatable `--proper-noun`).
- Unicode-aware casing (`é`, `ß`, CJK text, etc.) and Unicode word boundaries.
- Preservation of whitespace, punctuation, blank lines, and line endings.
- CLI with stdin/file input, stdout/file output, and per-mode operation.
- GUI with input/output text areas, one direct conversion button per mode,
  proper-noun input, and **Copy output**.
- Core unit tests, CLI integration tests, and frontend Vitest tests for
  feature-local parsing helpers.
- Mode-button-driven conversion (not live conversion while typing).

### Out of scope (v1)

- Part-of-speech tagging / NLP. Title Case is a deterministic heuristic, not a
  linguistic analyzer (see 3.7).
- Automatic detection of proper nouns. Sentence case and Title Case only apply
  the user-supplied list (Sentence case also capitalizes the first word of each
  sentence).
- Acronym and internal-capital preservation. For example, `NASA` becomes
  `Nasa` in Title Case and Capitalized Case unless it is added to the
  Sentence/Title proper-noun list. This can be revisited later.
- `tOGGLE cASE`, `snake_case`, `kebab-case`, `camelCase`, and `PascalCase`.
- Locale-specific casing (for example Turkish dotless `i`). Rust's default
  Unicode case mappings are used.
- Live/debounced conversion as the user types.
- CLI proper-noun files, comma-separated noun lists, or shell-completion
  integration.
- Multi-word proper-noun entries with internal punctuation (for example
  `St. Louis`) and hyphenated names (`Jean-Luc`). Single-token forms such as
  `O'Brien` and `U.S.A.` are supported.

## 3. Core Design (`crates/doctk-core/src/features/case_converter.rs`)

### 3.1 Public API

```rust
pub const MANIFEST: ToolManifest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseMode {
    Sentence,
    Lower,
    Upper,
    Capitalized,
    Title,
}

/// Convert `text` using `mode`. `proper_nouns` is used by `Sentence` and
/// `Title`. Conversion is infallible: every input string is valid.
pub fn convert(text: &str, mode: CaseMode, proper_nouns: &[String]) -> String;

pub fn sentence_case(text: &str, proper_nouns: &[String]) -> String;
pub fn lower_case(text: &str) -> String;
pub fn upper_case(text: &str) -> String;
pub fn capitalized_case(text: &str) -> String;
pub fn title_case(text: &str, proper_nouns: &[String]) -> String;

/// Minor words kept lowercase in the middle of a Title Case line.
pub const TITLE_MINOR_WORDS: &[&str];
/// Words with at least this many letters are always capitalized in Title Case.
pub const TITLE_LONG_WORD_MIN_CHARS: usize = 4;
```

`convert` is the single dispatcher used by the CLI and Tauri layers. The
mode-specific functions keep the core API convenient to test and reuse.

### 3.2 Manifest

```rust
pub const MANIFEST: ToolManifest = ToolManifest {
    id: "case_converter",
    name: "Case Converter",
    description: "Convert text to sentence case, lower case, UPPER CASE, Capitalized Case, or Title Case.",
    keywords: &["case", "convert", "sentence", "title", "upper", "lower", "capitalize", "text"],
    version: "0.1.0",
};
```

### 3.3 Shared Unicode handling

Add `unicode-segmentation = "1"` to `crates/doctk-core/Cargo.toml` and use its
UAX #29 word boundaries. The crate is already present in `Cargo.lock` as a
transitive dependency, so this does not add a new download.

Helpers (private unless noted otherwise):

- Word-token iteration uses `split_word_bounds` / `split_word_bound_indices`;
  only segments containing a cased letter are transformed and all separators
  (spaces, punctuation, newlines) are copied through untouched.
- `capitalize_first_cased(token: &str) -> String` — lowercase the token, then
  uppercase its first cased character. If the token has no cased character, it
  is returned lowercased unchanged. This handles `don't` → `Don't`,
  `3d` → `3D`, and `ß` expansion correctly. Ordinal suffixes are left
  lowercase (`4th`, `21st`) so the helper does not produce `4Th` or `21St`.
- `contains_cased_char(s: &str) -> bool` and
  `count_alphabetic(s: &str) -> usize` — used by Title Case rules.
- `lower_case` / `upper_case` call `str::to_lowercase` / `str::to_uppercase`
  directly; line endings and punctuation are preserved.

All casing uses Rust's default Unicode mappings, not a locale.

### 3.4 Sentence case

Algorithm:

1. **Find sentence segments with a small deterministic scanner on the original
   text.** UAX #29 sentence boundaries are not used because they depend on the
   input's existing capitalization and fail on all-lowercase input. The scanner
   treats these positions as sentence boundaries:
   - the start of the text and every newline;
   - after a run of sentence terminators (`.`, `!`, `?`, `…`, `。`, `！`, `？`)
     followed by zero or more closing quotes/brackets and zero or more
     whitespace characters, when the next character is a letter.
   - Guards: do not split on a decimal point between digits (`3.14`), after a
     common abbreviation (`Mr`, `Mrs`, `Ms`, `Dr`, `Prof`, `St`, `vs`, `etc`,
     `e.g`, `i.e`), after a single-letter initial, or inside an initialism such
     as `U.S.A.`.
2. **Lowercase each segment** independently, preserving its exact length and
   separators.
3. **Capitalize the first word.** Find the first UAX #29 word token in the
   segment. If that token contains a cased letter, uppercase its first cased
   letter; otherwise leave the whole segment unchanged. This means
   `123 hello world` stays `123 hello world` (the first word has no letter),
   while `"hello," she said` becomes `"Hello," she said`.
4. **Concatenate the segments.**
5. **Apply proper nouns** to the result (3.8).

Each line starts a new sentence because newlines are boundaries.

### 3.5 lower case and UPPER CASE

- `lower_case`: `text.to_lowercase()`.
- `upper_case`: `text.to_uppercase()`.

Both operate on the whole string, so whitespace, punctuation, and line endings
are preserved. Unicode expansions such as `ß` → `SS` are expected.

### 3.6 Capitalized Case

1. Lowercase the whole text.
2. For every UAX #29 word token, apply `capitalize_first_cased`.
3. Copy all separators unchanged.

Examples: `hello WORLD` → `Hello World`;
`don't state-of-the-art` → `Don't State-Of-The-Art`;
`3d printing` → `3D Printing`. All-caps acronyms are normalized
(`NASA` → `Nasa`).

### 3.7 Title Case

Title Case is a deterministic approximation of the requested behavior. It does
not use a part-of-speech tagger; instead, all non-minor words are treated as
major words (nouns, verbs, adjectives, adverbs), which is the standard
editorial approach to title casing.

Rules:

1. Split the input into lines and process each non-empty line as an independent
   title. Empty lines and lines with no cased words are unchanged.
2. Tokenize the line with UAX #29 word boundaries.
3. Identify the first and last word token that contains a cased letter. These
   are always capitalized, even if they are minor words.
4. For every other cased word token:
   - If it immediately follows a colon (`:`), em dash (`—`), or en dash (`–`),
     capitalize it (it begins a new title section, for example after a
     subtitle colon).
   - Else if its lowercased form is in `TITLE_MINOR_WORDS`, lowercase the whole
     token.
   - Else if it has at least `TITLE_LONG_WORD_MIN_CHARS` (4) alphabetic
     letters, capitalize it.
   - Else capitalize it (short content words are treated as major words).
5. `capitalize` means: lowercase the token, then uppercase its first cased
   character.
6. Apply proper nouns to the converted text (3.8), so canonical spellings such
   as `NASA`, `iPhone`, and `New York` override the title-case result.
7. Preserve every separator (spaces, punctuation, newlines) exactly.

`TITLE_MINOR_WORDS` (articles, conjunctions, and short prepositions) is:

```text
a, an, and, as, at, but, by, for, if, in, nor, of, off, on, or,
per, so, the, to, up, via, vs, yet
```

The list contains only words of three letters or fewer so that the long-word
rule never conflicts with it. Longer prepositions such as `with`, `from`, and
`over` are capitalized because they are long words.

Examples:

```text
the lord of the rings          -> The Lord of the Rings
a tale of two cities           -> A Tale of Two Cities
war and peace                  -> War and Peace
to kill a mockingbird          -> To Kill a Mockingbird
of mice and men                -> Of Mice and Men
something to believe in        -> Something to Believe In
star wars: a new hope          -> Star Wars: A New Hope
the state-of-the-art method    -> The State-of-the-Art Method
THE LORD OF THE RINGS          -> The Lord of the Rings
the lord of the rings\nof mice and men
                               -> The Lord of the Rings\nOf Mice and Men
```

### 3.8 Proper-noun matching (Sentence case and Title Case)

In Sentence case the replacements are applied after sentence capitalization.
In Title Case they are applied after the title-case rules. Input list:

- GUI: one entry per line; entries are trimmed and empty lines ignored.
- CLI: one `--proper-noun` flag per entry, repeatable.
- Core: `&[String]`, so the parsing source does not matter.

Matching rules:

1. Normalize each entry: trim, split into UAX #29 word tokens, drop entries
   with no words, and keep the original token spelling as the canonical form.
2. Match case-insensitively: a text word token matches a noun word token when
   their lowercased forms are equal.
3. Multi-word entries match consecutive text word tokens when the separators
   between those tokens contain only whitespace. This makes `New York` match
   `new york` and `new   york`, and preserves the original spacing in the
   output.
4. At each text position, try the longest matching noun first (most words, then
   most characters), so `New York` wins over `New`. Replace every matched word
   with the corresponding canonical word. Matching is non-overlapping.
5. Boundaries are whole-word because matching is done on word tokens:
   `Ann` does not match `annex` or `planning`.
6. Duplicate entries (case-insensitive) are ignored after the first.

Examples with the noun list `["John", "New York", "NASA", "iPhone"]`:

```text
john met mary in new york          -> John met mary in New York
iphone and nasa are here           -> iPhone and NASA are here
JOHN and jOhN                      -> John and John
ann planning annex (list: Ann)     -> Ann planning annex
nasa and the space race (Title Case) -> NASA and the Space Race
```

The canonical spelling from the user list is always used. That is why
`nasa` becomes `NASA` and `iphone` becomes `iPhone`.

Known limitation: multi-word entries with internal punctuation (`St. Louis`)
and hyphenated names (`Jean-Luc`) are not matched as phrases in v1.
Single-token forms such as `O'Brien` and `U.S.A.` work because UAX #29 keeps
each as one word token.

### 3.9 Dispatch

```rust
pub fn convert(text: &str, mode: CaseMode, proper_nouns: &[String]) -> String {
    match mode {
        CaseMode::Sentence => sentence_case(text, proper_nouns),
        CaseMode::Lower => lower_case(text),
        CaseMode::Upper => upper_case(text),
        CaseMode::Capitalized => capitalized_case(text),
        CaseMode::Title => title_case(text, proper_nouns),
    }
}
```

`proper_nouns` is ignored by all modes except `Sentence` and `Title`.

## 4. CLI Design (`doctk-cli/src/commands/case_converter.rs`)

```bash
doctk case <MODE> [INPUT] [-o OUTPUT] [--proper-noun <NOUN>]...
```

- `MODE` is required and accepts one of:
  `sentence`, `lower`, `upper`, `capitalized`, `title`.
- `INPUT` defaults to stdin (`-`); `OUTPUT` defaults to stdout (`-`).
- `--proper-noun <NOUN>` is repeatable and valid with `sentence` and `title`
  modes. Passing it with another mode returns `DoctkError::InvalidArgument`
  and exit code `1`.
- UTF-8 text only.
- Exit codes: `0` success; `1` processing/argument error; `2` clap usage
  error (invalid mode, unknown flag).

Examples:

```bash
echo "hello WORLD" | doctk case upper
# HELLO WORLD

doctk case sentence notes.txt --proper-noun John --proper-noun "New York"
# writes converted text to stdout

doctk case title headings.txt -o headings-out.txt

echo "nasa and the space race" | doctk case title - --proper-noun NASA
# NASA and the Space Race
```

Register the subcommand in `commands/mod.rs` as `case`, after `diff` in
`build_cli()` and `dispatch()`.

## 5. GUI Design (`gui/src/features/case-converter/`)

### 5.1 Layout

```text
[ Input text                                                                ]
[ [ textarea (monospace, scrollable, default height H)                     ] ]
[ [ Sentence case ] [ lower case ] [ UPPER CASE ]                          ]
[ [ Capitalized Case ] [ Title Case ]                                      ]
--------------------------------------------------------------------------------
[ Output text                                                               ]
[ [ textarea (read-only, monospace, scrollable, default height H)           ] ]
[ [ Copy output ]   status/copy message                                     ]
--------------------------------------------------------------------------------
[ Proper nouns (used by Sentence case and Title Case) — only shown when     ]
[ Sentence case or Title Case is active                                     ]
[ [ small textarea: John                                                   ] ]
[ [                  New York                                              ] ]
[ [                  NASA                                                  ] ]
[ [ hint: one proper noun or phrase per line                               ] ]
```

- Input starts with a short sample (for example
  `hello world. this is a test.`) and output starts empty.
- There is **no mode dropdown, no separate Convert section, and no mode hint
  line**. The five mode buttons live directly under the input textarea inside
  the input section.
- Each mode button converts immediately when clicked:
  `Sentence case`, `lower case`, `UPPER CASE`, `Capitalized Case`,
  `Title Case`. Each button keeps its `title` tooltip from `CASE_MODES`.
- The last-clicked mode button is visually highlighted so the user can see
  which conversion produced the current output. This is only an active-state
  indicator; it is not a selector.
- The proper-noun section is rendered **only when Sentence case or Title Case
  is the active mode**. It is hidden before the first conversion and while
  `lower case`, `UPPER CASE`, or `Capitalized Case` is active.
- The proper-noun section is placed at the **bottom of the tool, below the
  output textarea**. Its placeholder is `John\nNew York\nNASA` and its hint
  says: “One proper noun or phrase per line. Capitalization is copied to the
  output; click the active mode button again to apply changes.”
- To apply newly entered proper nouns, the user clicks the active
  Sentence/Title button again. There is no auto-conversion.
- The output text area is read-only and selectable, with the same default
  height as the input.
- **Copy output** writes the full output to the clipboard with
  `navigator.clipboard.writeText`.
- Conversion errors are shown inline below the output toolbar.

### 5.2 Behavior

- Clicking a mode button converts the current input immediately:
  1. Split the proper-noun text area on newlines, trim each entry, and drop
     empties.
  2. Call the Tauri command with the clicked mode and the parsed proper-noun
     list.
  3. Replace the output with the result, mark that mode button active, and
     show the proper-noun section if the active mode is Sentence case or
     Title Case.
- The proper-noun list is sent with every conversion call; the core ignores it
  for `lower`, `upper`, and `capitalized`.
- There is no auto-conversion while typing and no generic **Convert** button.
  Conversion is always an explicit mode-button click.
- Clicking another mode button re-converts the same input without changing the
  input or proper-noun list, and updates the visibility of the proper-noun
  section.
- Switching to another tool tab and back preserves all component state because
  the shell keeps tool components mounted with `hidden`.
- The GUI never transforms text itself; it only calls the core conversion via
  Tauri.
- Feature-local pure helpers live in `caseModes.ts`:
  `CASE_MODES` (labels, descriptions, and button order),
  `showProperNouns(mode)` (visibility rule), and
  `parseProperNounInput(input: string): string[]`.

### 5.3 Tauri command

```rust
#[tauri::command]
pub fn case_converter_convert(
    text: String,
    mode: CaseMode,
    proper_nouns: Vec<String>,
) -> Result<String, String> {
    Ok(case_converter::convert(&text, mode, &proper_nouns))
}
```

- Command name is prefixed with the tool id.
- Conversion is infallible, but the command returns `Result<String, String>` to
  match the other Tauri commands and to leave room for future validation.
- The frontend wrapper in `gui/src/lib/tauri.ts`:

```ts
export type CaseMode = "sentence" | "lower" | "upper" | "capitalized" | "title";

export async function convertCase(
  text: string,
  mode: CaseMode,
  properNouns: string[],
): Promise<string> {
  return invoke<string>("case_converter_convert", { text, mode, properNouns });
}
```

Tauri maps the camelCase `properNouns` argument to the Rust `proper_nouns`
parameter.

### 5.4 Registration order

- `gui/src/toolRegistry.tsx`: insert the Case Converter entry **immediately
  after** `diff_checker`, before `markdown_tsv`.
- `crates/doctk-core/src/registry.rs`: insert
  `crate::features::case_converter::MANIFEST` after `diff_checker` for a
  consistent order across layers.
- `crates/doctk-cli/src/commands/mod.rs`: register `case` after `diff`.
- `gui/src-tauri/src/features/mod.rs`: add
  `case_converter::case_converter_convert` to the handler list.
- Suggested icon: `🔤` (Input Latin Letters), matching the emoji style used by
  the sidebar.

## 6. Edge Cases

| Case | Expected behavior |
|---|---|
| Empty input | Empty output for every mode |
| Whitespace/punctuation/emoji only | Unchanged (no cased letters to convert) |
| First word starts with a digit (`123 hello`) | Sentence case leaves the sentence unchanged |
| Ordinal word (`4th`, `21ST`) | Suffix stays lowercase (`4th`, `21st`) |
| Decimal number (`3.14 is pi`) | Sentence boundary detector does not split the decimal |
| Common abbreviation (`e.g. this is an example`) | Not split after `e.g`; no `This` capitalization |
| Leading quote (`"hello," she said`) | First word `hello` is capitalized inside the quotes |
| Newlines | Preserved; each new line starts a new Sentence-case sentence |
| CRLF input | Preserved |
| CJK / uncased scripts | Left unchanged; only cased scripts are converted |
| Unicode expansion (`ß`) | `UPPER CASE` may expand to `SS` |
| Proper noun partial word (`Ann` vs `annex`) | No match |
| Overlapping nouns (`New` and `New York`) | Longest phrase wins |
| Duplicate/blank proper-noun entries | Ignored |
| Proper noun at sentence start (`nasa`) | Canonical user form wins (`NASA`) |
| Proper noun in Title Case (`nasa and the space race`, list `NASA`) | Title rules run first, then canonical form wins (`NASA and the Space Race`) |
| All-caps title input (`THE LORD OF THE RINGS`) | Title Case lowercases and applies the rules |
| Hyphenated words in Title Case (`state-of-the-art`) | Each hyphen-separated part is treated as a word; minor parts stay lowercase |
| All-caps acronyms (`NASA`) | Normalized (`Nasa`) in v1 unless listed as a proper noun; documented limitation |
| Multi-line Title Case input | Each non-empty line is a separate title |
| Very large input | Linear scan; no size limit in v1 |

## 7. Testing

### Core (unit tests in `case_converter.rs`)

- `lower_case` / `upper_case`: ASCII, punctuation, Unicode (`Straße`,
  `ÉCOLE`), newlines, empty input.
- Sentence case:
  - first word of one sentence, multiple sentences, and newline-separated lines;
  - all-uppercase and all-lowercase input;
  - leading quotes and punctuation;
  - first word without a cased letter (`123 hello`);
  - decimal and abbreviation guards (`3.14`, `e.g.`, `Dr.`);
  - proper nouns: single word, multi-word, canonical casing, case-insensitive
    match, whole-word boundaries, overlap resolution, duplicate/blank entries,
    empty list, apostrophe/initialism forms (`O'Brien`, `U.S.A.`).
- Capitalized Case:
  - `hello WORLD` → `Hello World`;
  - apostrophes (`don't` → `Don't`), hyphens
    (`state-of-the-art` → `State-Of-The-Art`), digits (`3d` → `3D`);
  - ordinal suffixes stay lowercase (`4th` → `4th`, not `4Th`);
  - Unicode and punctuation.
- Title Case:
  - minor words in the middle, first, and last positions;
  - long words (4+ letters) capitalized;
  - `the lord of the rings`, `to kill a mockingbird`,
    `star wars: a new hope`, `the state-of-the-art method`;
  - all-caps input, multi-line input, empty/punctuation-only lines;
  - punctuation and apostrophes;
  - proper nouns override title casing (`nasa` → `NASA`, `iphone` → `iPhone`).
- `convert` dispatches to the correct mode and ignores `proper_nouns` for
  `lower`, `upper`, and `capitalized`.
- Infallible behavior for empty, whitespace-only, emoji-only, and CJK input.

### CLI (`crates/doctk-cli/tests/cli.rs`)

- stdin → stdout for `upper`, `lower`, and `sentence`.
- File input with `-o` output.
- `sentence` and `title` with repeatable `--proper-noun`.
- Invalid mode exits with code `2` (clap).
- `--proper-noun` with `lower`, `upper`, or `capitalized` exits with code `1`
  and a clear error message.
- `doctk case --help` lists the command and modes.

### Frontend (Vitest)

- `parseProperNounInput` splits on newlines, trims, and removes empty entries.
- `showProperNouns` returns true only for `sentence` and `title`.
- `CASE_MODES` contains the five labels/descriptions in display order.

### GUI (manual smoke matrix)

- Sidebar shows Case Converter immediately after Diff Checker.
- Click each of the five mode buttons and verify that the output updates
  immediately, with no dropdown, no separate **Convert** section, and no mode
  hint line.
- The proper-noun section is hidden before the first conversion and for
  `lower`, `upper`, and `capitalized`; it appears at the bottom of the tool
  (below the output textarea) for `sentence` and `title`.
- Sentence case and Title Case with a proper-noun list produce the canonical
  spelling.
- The active mode button is highlighted after conversion.
- **Copy output** copies the full converted text.
- Switching tools and returning preserves input, active mode indicator,
  proper nouns, and output.

## 8. Registration Checklist

Per `docs/tool-framework.md`:

- [ ] Core: `crates/doctk-core/src/features/case_converter.rs`
  - `MANIFEST`, `CaseMode`, mode functions, `convert`, unit tests
  - register in `features/mod.rs` and `registry.rs` (after `diff_checker`)
- [ ] Core dependency: add `unicode-segmentation = "1"` to
  `crates/doctk-core/Cargo.toml`
- [ ] CLI: `crates/doctk-cli/src/commands/case_converter.rs`
  - `cli()` and `run()`
  - register in `commands/mod.rs` after `diff`
  - integration tests in `crates/doctk-cli/tests/cli.rs`
- [ ] Tauri: `gui/src-tauri/src/features/case_converter.rs`
  - `case_converter_convert`
  - register in `features/mod.rs`
- [ ] Frontend: `gui/src/features/case-converter/`
  - `index.tsx`, `caseModes.ts`, `caseModes.test.ts`
  - register in `gui/src/toolRegistry.tsx` immediately after `diff_checker`
  - add `CaseMode` and `convertCase` to `gui/src/lib/tauri.ts`
- [ ] Docs: add this file to `docs/PLAN.md`, add a row to the README feature
  table, and update the feature/dependency examples in
  `docs/tool-framework.md` where needed
- [ ] Verify `doctk --help`, `doctk case --help`, and GUI sidebar order

## 9. Design Decisions for Review

These are the interpretation choices baked into this plan. Please confirm or
request changes before implementation:

1. **Sentence boundaries.** Sentence case capitalizes the first word of each
   sentence, not only the first word of the whole text. Boundaries are
   best-effort (terminators, newlines, decimal/abbreviation guards).
2. **Proper-noun list.** Entries are matched case-insensitively as whole words
   or whitespace-separated phrases, and the user's exact spelling is written to
   the output (`New York`, `NASA`, `iPhone`). The list is applied to Sentence
   case and Title Case, after each mode's own casing rules.
3. **Title Case heuristic.** Without a POS tagger, every non-minor word is
   treated as a major word. The minor-word list, the 4+ letter long-word rule,
   first/last-word capitalization, and the colon/dash rule implement the
   requested behavior. Each non-empty line is treated as a separate title.
4. **Acronyms.** Capitalized Case normalizes acronyms (`NASA` → `Nasa`). Title
   Case also normalizes them unless they are supplied as proper nouns; general
   acronym preservation is a future extension.
5. **No new error type.** Conversion is infallible. CLI misuse of
   `--proper-noun` with an unsupported mode is reported as an invalid
   argument.
6. **Direct mode buttons.** The GUI uses one button per mode (no dropdown, no
   separate **Convert** section, and no mode hint line). Clicking a mode button
   converts immediately. The proper-noun section is placed below the output
   textarea and is only rendered when Sentence case or Title Case is active.

## 10. Future Extensions

- Automatic acronym/proper-name preservation in Title Case (dictionary or
  mixed-case heuristic, beyond the user-supplied list).
- True part-of-speech tagging for noun/verb/adjective detection.
- User-configurable minor-word list and long-word threshold.
- Multi-word and hyphenated proper-noun matching (`Jean-Luc`,
  `St. Louis`).
- CLI `--proper-nouns-file` and comma-separated list support.
- Additional cases: `tOGGLE cASE`, `snake_case`, `kebab-case`, `camelCase`,
  `PascalCase`.
- Locale-aware casing (for example Turkish `i`/`İ`).
- Live conversion as the user types (debounced).
