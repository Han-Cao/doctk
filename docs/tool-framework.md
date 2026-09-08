# Tool Framework Design Plan

This document defines the overall architecture and the framework conventions used to build and extend the document processing tool. It is intentionally feature-agnostic; individual feature plans live in separate files and plug into the framework described here.

## 1. Goals

1. **Shared logic, thin front-ends.** All document-processing logic lives in a UI-agnostic Rust core library. CLI and GUI are adapters that call the same code.
2. **A feature is a plugin.** Adding a new tool means adding one feature module in each layer and registering it — no modification of shared shells or navigation.
3. **Predictable layer contract.** Every feature follows the same pattern across core, CLI, Tauri backend, and web frontend.
4. **Cross-platform.** GUI on Windows/Linux/macOS; CLI on Linux/macOS.
5. **Testable.** Each layer can be tested independently; feature tests live next to the feature.

## 2. Architecture Overview

```text
┌────────────────────────────────────────────────────────────────────────┐
│                          GUI (Tauri 2 + React)                        │
│                                                                       │
│  gui/src/                                                             │
│    ├── shell/              # app shell: sidebar, routing              │
│    ├── toolRegistry.tsx    # feature registry for the UI              │
│    ├── lib/tauri.ts        # typed invoke() wrappers                  │
│    └── features/<feature>/ # one folder per tool (components, UI logic)│
│                                                                       │
│  gui/src-tauri/src/                                                   │
│    ├── features/<feature>/ # one module per tool: Tauri commands       │
│    ├── features/mod.rs     # aggregates all feature command handlers   │
│    └── lib.rs              # app setup; calls feature registration     │
└────────────────────────────────────────────────────────────────────────┘
                              │ invoke()
                              ▼
┌────────────────────────────────────────────────────────────────────────┐
│                      crates/doctk-core (Rust library)                 │
│                                                                       │
│  src/                                                                 │
│    ├── registry.rs         # ToolManifest + feature registry          │
│    ├── error.rs            # shared typed errors                      │
│    ├── units.rs            # mm/cm/in/pt conversion helpers           │
│    └── features/<feature>/ # one module per tool: pure logic          │
└────────────────────────────────────────────────────────────────────────┘
                              │ shared crate dependency
                              ▼
┌────────────────────────────────────────────────────────────────────────┐
│                       crates/doctk-cli (clap binary)                  │
│                                                                       │
│  src/                                                                 │
│    ├── main.rs             # shell: builds clap app from registry     │
│    └── commands/<feature>/ # one module per tool: clap Command + run  │
└────────────────────────────────────────────────────────────────────────┘
```

### Why this shape?

- `doctk-core` has no GUI/CLI dependencies, so it can be tested quickly and reused by any future front-end.
- The GUI shell and CLI shell know nothing about a specific tool except what the registry tells them.
- A feature's layers are physically co-located by name (`diff_checker`, `case_converter`, `markdown_tsv`, `markdown_text`, `pdf_checker`) so a new feature is a copy-paste template plus logic.

## 3. Workspace Layout

```text
doctk/
├── Cargo.toml                      # workspace: doctk-core, doctk-cli, doctk-gui
├── Cargo.lock
├── README.md
├── docs/
│   ├── PLAN.md                     # plan index
│   ├── DEVELOPMENT.md              # dependencies, build, and development guide
│   ├── tool-framework.md
│   ├── feature-diff-checker.md
│   ├── feature-markdown-tsv.md
│   ├── feature-markdown-text.md
│   └── feature-pdf-checker.md
├── scripts/
│   ├── build-all.sh / build-all.ps1
│   ├── build-cli.sh / build-cli.ps1
│   └── build-gui.sh / build-gui.ps1
├── crates/
│   ├── doctk-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs
│   │       ├── registry.rs
│   │       ├── units.rs
│   │       └── features/
│   │           ├── mod.rs
│   │           ├── case_converter.rs
│   │           ├── diff_checker.rs
│   │           ├── markdown_text.rs
│   │           ├── markdown_tsv.rs
│   │           └── pdf_checker.rs
│   └── doctk-cli/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── main.rs
│       │   └── commands/
│       │       ├── mod.rs
│       │       ├── case_converter.rs
│       │       ├── diff_checker.rs
│       │       ├── markdown_text.rs
│       │       ├── markdown_tsv.rs
│       │       └── pdf_checker.rs
│       └── tests/
│           ├── cli.rs
│           └── fixtures/
└── gui/
    ├── package.json
    ├── package-lock.json
    ├── index.html
    ├── vite.config.ts
    ├── tsconfig.json
    ├── src/
    │   ├── main.tsx
    │   ├── App.tsx
    │   ├── styles.css
    │   ├── shell/
    │   │   └── ToolLayout.tsx
    │   ├── toolRegistry.tsx
    │   ├── lib/
    │   │   └── tauri.ts
    │   └── features/
    │       ├── diff-checker/
    │       │   └── index.tsx
    │       ├── case-converter/
    │       │   ├── index.tsx
    │       │   ├── caseModes.ts
    │       │   └── caseModes.test.ts
    │       ├── markdown-text/
    │       │   └── index.tsx
    │       ├── markdown-tsv/
    │       │   ├── index.tsx
    │       │   ├── gridOps.ts
    │       │   └── gridOps.test.ts
    │       └── pdf-checker/
    │           └── index.tsx
    └── src-tauri/
        ├── Cargo.toml
        ├── build.rs
        ├── tauri.conf.json
        ├── capabilities/
        │   └── default.json
        ├── icons/
        │   ├── icon.png
        │   └── icon.ico
        └── src/
            ├── main.rs
            ├── lib.rs
            └── features/
                ├── mod.rs
                ├── case_converter.rs
                ├── diff_checker.rs
                ├── markdown_text.rs
                ├── markdown_tsv.rs
                └── pdf_checker.rs
```

## 4. Core Framework: Registry & Feature Contract

### 4.1 Tool manifest

`doctk-core/src/registry.rs` defines the canonical identity of every tool:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ToolManifest {
    pub id: &'static str,            // stable machine id, e.g. "markdown_text"
    pub name: &'static str,          // display name, e.g. "Markdown → Text"
    pub description: &'static str,   // one-line description for tooltips/menus
    pub keywords: &'static [&'static str], // future search/filter
    pub version: &'static str,       // feature version
}

pub const TOOL_REGISTRY: &[ToolManifest] = &[
    crate::features::diff_checker::MANIFEST,
    crate::features::case_converter::MANIFEST,
    crate::features::markdown_tsv::MANIFEST,
    crate::features::markdown_text::MANIFEST,
    crate::features::pdf_checker::MANIFEST,
];
```

Rules:

- `id` is stable and used by CLI subcommands, Tauri command prefixes, and frontend routes. Do not rename after release.
- Core feature modules export a `MANIFEST: ToolManifest` constant.
- `TOOL_REGISTRY` is the single source of truth for "what tools exist" in core. CLI and GUI wrap this with additional rendering data (icons, routes).

### 4.2 Core feature module contract

Each `doctk-core/src/features/<feature>.rs` must:

- Export a `pub const MANIFEST: ToolManifest`.
- Expose pure, UI-agnostic functions. Most return `Result<T, DoctkError>`; infallible converters may return `String` directly (see `markdown_text`).
- Not depend on `clap`, `tauri`, or any frontend crate.
- Serialize all output models with `serde::{Serialize, Deserialize}` where the model crosses the GUI/CLI boundary.
- Keep feature-specific tests in the same file under `#[cfg(test)]`. CLI fixture tests live under `crates/doctk-cli/tests/fixtures/`.

### 4.3 Shared core services

Small cross-feature services live at the core root:

| Module | Responsibility |
|---|---|
| `error.rs` | `DoctkError` enum with feature-specific variants (`Table`, `Diff`, `Pdf`, `Io`, `InvalidArgument`) |
| `registry.rs` | `ToolManifest`, `TOOL_REGISTRY` |
| `units.rs` | Unit conversion (`mm`, `cm`, `in`, `pt`) and paper-size presets used by PDF checker and future print tools |

## 5. CLI Framework

### 5.1 Shell

`doctk-cli/src/main.rs` builds the top-level command:

```text
doctk
├── diff        # from feature diff_checker
├── case        # from feature case_converter
├── table       # from feature markdown_tsv
├── md2text     # from feature markdown_text
└── pdf         # from feature pdf_checker
```

The shell does:

1. Build the top-level `clap::Command` with version and help text.
2. Collect subcommands from `commands::subcommands()`.
3. Match and dispatch to the appropriate feature runner.

### 5.2 Feature command contract

Each `doctk-cli/src/commands/<feature>.rs` must export:

```rust
pub fn cli() -> clap::Command;                          // feature-specific clap definition
pub fn run(matches: &clap::ArgMatches) -> Result<i32, DoctkError>; // parse args, call core, format output
```

`commands/mod.rs` aggregates:

```rust
pub fn build_cli() -> clap::Command {
    clap::Command::new("doctk")
        .subcommand(diff_checker::cli())
        .subcommand(case_converter::cli())
        .subcommand(markdown_tsv::cli())
        .subcommand(markdown_text::cli())
        .subcommand(pdf_checker::cli())
}

pub fn dispatch(matches: &ArgMatches) -> Result<i32, DoctkError> {
    match matches.subcommand() {
        Some(("diff", sub)) => diff_checker::run(sub),
        Some(("case", sub)) => case_converter::run(sub),
        Some(("table", sub)) => markdown_tsv::run(sub),
        Some(("md2text", sub)) => markdown_text::run(sub),
        Some(("pdf", sub)) => pdf_checker::run(sub),
        _ => Err(DoctkError::InvalidArgument(...)),
    }
}
```

Adding a CLI feature = add module + register it in `build_cli()` and `dispatch()`.

### 5.3 Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Processing error (parse, I/O, unsupported input) |
| 2 | CLI usage error (handled by clap) |
| 3 | Tool-specific "check failed" (e.g. PDF page does not fit) |

## 6. GUI Framework

### 6.1 Shell

The React shell (`gui/src/App.tsx` + `gui/src/shell/ToolLayout.tsx`) renders:

- **Sidebar**: generated from `toolRegistry.tsx`, not hard-coded in the shell.
- **Main area**: renders all registered tool components, toggling visibility with the `hidden` attribute so tab switching preserves component state.
- **Routing**: state-based active tool id; the route path is the tool id.
- **State preservation**: switching between tools keeps every tool mounted (hidden with `hidden`), so switching tabs never resets a tool to its initial sample data.

### 6.2 Frontend feature contract

`gui/src/toolRegistry.tsx`:

```ts
export interface ToolDefinition {
  id: string;                 // must match ToolManifest.id in core
  name: string;
  description: string;
  icon: string;               // emoji icon used by the sidebar
  component: React.LazyExoticComponent<React.ComponentType>;
}

export const TOOL_DEFINITIONS: ToolDefinition[] = [
  { id: 'diff_checker', ..., component: lazy(() => import('./features/diff-checker')) },
  { id: 'case_converter', ..., component: lazy(() => import('./features/case-converter')) },
  { id: 'markdown_tsv', ..., component: lazy(() => import('./features/markdown-tsv')) },
  { id: 'markdown_text', ..., component: lazy(() => import('./features/markdown-text')) },
  { id: 'pdf_checker', ..., component: lazy(() => import('./features/pdf-checker')) },
];
```

Rules:

- Each `gui/src/features/<feature>/index.tsx` default-exports a React component.
- The feature component owns its own state and UI logic; the shell provides only layout.
- Tool-specific Tauri calls are wrapped in `gui/src/lib/tauri.ts` (typed `invoke` wrappers).
- Feature-local pure logic should live in separate files (e.g. `gridOps.ts`) with Vitest tests.

### 6.3 Tauri backend feature contract

Each `gui/src-tauri/src/features/<feature>.rs` exposes Tauri command functions:

```rust
#[tauri::command]
pub fn markdown_text_convert(md: String) -> Result<String, String> { ... }

#[tauri::command]
pub async fn pdf_checker_check_files(...) -> Result<Vec<PdfFileReport>, String> { ... }
```

`features/mod.rs` aggregates handlers:

```rust
pub fn handlers() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        markdown_tsv::markdown_tsv_md_to_table,
        markdown_tsv::markdown_tsv_parse_tsv,
        markdown_tsv::markdown_tsv_table_to_md,
        markdown_text::markdown_text_convert,
        diff_checker::diff_checker_side_by_side,
        diff_checker::diff_checker_track_changes,
        case_converter::case_converter_convert,
        pdf_checker::pdf_checker_check_files,
    ]
}
```

`lib.rs` registers `handlers()` and the minimal plugins (`dialog`, `fs` for user-selected files only).

Adding a GUI feature = add frontend folder + Tauri feature module + register in `features/mod.rs` and `toolRegistry.tsx`.

### 6.4 Backend command conventions

- Command names are prefixed by tool id to avoid collisions, e.g. `markdown_tsv_md_to_table`, `pdf_checker_check_files`.
- Commands receive/return JSON-serializable core types. The frontend never parses/renders backend-specific formats.
- Long-running operations (PDF batch checks) are `async`; heavy CPU work is moved to `tauri::async_runtime::spawn_blocking`.
- Every command maps core errors to `String` via `DoctkError::to_user_message()`.

## 7. Error Handling Framework

```rust
pub enum DoctkError {
    Table(TableError),
    Diff(DiffError),
    Pdf(PdfError),
    Io(std::io::Error),
    InvalidArgument(String),
}
```

- Core returns typed errors with actionable messages (e.g. `line 5: expected 3 columns, found 4`).
- CLI prints `error: <message>` to stderr and returns the appropriate exit code.
- GUI catches error strings from Tauri commands and shows them inline or as status messages; recoverable validation errors are shown inline by the feature component.

## 8. Logging & Diagnostics

- No global logging framework is configured yet.
- CLI errors are written to stderr.
- GUI errors are shown inline or as status text inside the tool panel.
- Future work: add `tracing`/`tracing-subscriber` with `RUST_LOG` support.

## 9. Testing Strategy

| Layer | Test type | Tooling |
|---|---|---|
| Core | Unit tests per feature | `cargo test -p doctk-core` |
| CLI | End-to-end command tests | `cargo test -p doctk-cli` (uses `CARGO_BIN_EXE_doctk`) |
| Frontend | Pure UI logic tests (`gridOps.ts`) | Vitest |
| GUI | Manual cross-platform smoke matrix | Windows/Linux/macOS |

## 10. Build & Distribution

- CLI binaries: `./scripts/build-cli.sh` (Linux/macOS) or `.\scripts\build-cli.ps1` (Windows) → `build/bin/doctk[.exe]`.
- GUI bundles: `./scripts/build-gui.sh` (Linux/macOS) or `.\scripts\build-gui.ps1` (Windows) → Tauri bundles under `build/cargo-target/release/bundle`.
- All build scripts set `CARGO_TARGET_DIR` to the root-level `build/cargo-target`; the frontend outputs to `build/gui-dist`.
- Default workspace members are `doctk-core` and `doctk-cli`, so plain `cargo test` does not require Tauri system libraries.
- CI matrix:
  - Linux + macOS: CLI build/test.
  - Linux + macOS + Windows: GUI build smoke test.
- All features ship in one binary/app; per-feature build isolation is not required.

## 11. Dependencies

| Crate / package | Purpose |
|---|---|
| `clap` | CLI parsing |
| `serde` + `serde_json` | Serialization between core, CLI, GUI |
| `thiserror` | Typed core errors |
| `similar` | Line and word diff |
| `unicode-segmentation` | Unicode word boundaries for case conversion |
| `lopdf` | PDF parsing |
| `tauri` 2.x | GUI shell |
| `tauri-plugin-dialog` | Open/save dialogs |
| `tauri-plugin-fs` | User-selected file read/write |
| `@tauri-apps/api` | Frontend `invoke` wrappers and drag-drop events |
| `@tauri-apps/plugin-dialog` | Frontend file dialogs |
| `@tauri-apps/plugin-fs` | Frontend file reads |
| Vite + React + TypeScript | Web frontend |
| Vitest | Frontend unit tests |
| Plain CSS (`gui/src/styles.css`) | Styling (shell + features) |

## 12. Extension Checklist (add a new feature)

1. **Core**
   - [ ] Create `crates/doctk-core/src/features/<feature_id>.rs`.
   - [ ] Define `pub const MANIFEST: ToolManifest`.
   - [ ] Implement pure functions with typed errors and serde models where needed.
   - [ ] Add `pub mod <feature_id>;` in `features/mod.rs` and add `MANIFEST` to `TOOL_REGISTRY`.
   - [ ] Add unit tests.
2. **CLI**
   - [ ] Create `crates/doctk-cli/src/commands/<feature_id>.rs`.
   - [ ] Implement `cli()` and `run()`.
   - [ ] Register in `commands/mod.rs` (`build_cli()` and `dispatch()`).
   - [ ] Add CLI tests in `crates/doctk-cli/tests/cli.rs`.
3. **Tauri backend**
   - [ ] Create `gui/src-tauri/src/features/<feature_id>.rs`.
   - [ ] Implement `#[tauri::command]` functions using the core API.
   - [ ] Register commands in `gui/src-tauri/src/features/mod.rs`.
4. **Frontend**
   - [ ] Create `gui/src/features/<feature-id>/index.tsx` and feature-specific files.
   - [ ] Add a `toolRegistry.tsx` entry with `id`, `name`, `description`, `icon`, `component`.
   - [ ] Add Vitest tests for feature-local pure logic.
5. **Release**
   - [ ] Update `docs/PLAN.md` index with the new feature doc.
   - [ ] Add a plan file `docs/feature-<feature-id>.md`.
   - [ ] Update `README.md` and `docs/DEVELOPMENT.md` as needed.
   - [ ] Verify CLI `--help` and GUI sidebar order.

## 13. Extension Scenarios (the framework must support)

- **New converter tool** (e.g. CSV ↔ JSON): follow the checklist; only the `doctk-core` feature module and GUI component are substantial.
- **New checker tool** (e.g. font/license checker for PDFs): reuse `units.rs` and PDF primitives; add a `pdf_*` tool under the same feature pattern.
- **Feature-specific settings**: each GUI feature owns its own settings panel; no shell changes needed.
- **CLI-only feature**: skip steps 3–4; the registry still advertises it for `--help` and future GUI use.
- **GUI-only feature**: skip step 2; still define core + Tauri command + frontend feature.
- **Feature deprecation**: remove entries from the registries; keep a tombstone in CLI `--help` if needed.

## 14. Non-Goals for v1

- Dynamic loading of shared libraries (plugins are compile-time).
- Third-party plugin marketplace.
- Per-feature installation/updates.
- Remote content or web services.
