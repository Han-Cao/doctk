# Tool Framework Design Plan

This document defines the overall architecture and the framework conventions used to build and extend the document processing tool. It is intentionally feature-agnostic; individual feature plans live in separate files and plug into the framework described here.

## 1. Goals

1. **Shared logic, thin front-ends.** All document-processing logic lives in a UI-agnostic Rust core library. CLI and GUI are adapters that call the same code.
2. **A feature is a plugin.** Adding a new tool should mean adding one feature module in each layer and registering it — no modification of shared shells or navigation.
3. **Predictable layer contract.** Every feature follows the same pattern across core, CLI, Tauri backend, and web frontend.
4. **Cross-platform.** GUI on Windows/Linux/macOS; CLI on Linux/macOS.
5. **Testable.** Each layer can be tested independently; feature tests live next to the feature.

## 2. Architecture Overview

```text
┌────────────────────────────────────────────────────────────────────────┐
│                          GUI (Tauri 2 + React)                        │
│                                                                       │
│  gui/src/                                                             │
│    ├── shell/              # app shell: sidebar, routing, toasts      │
│    ├── toolRegistry.tsx    # feature registry for the UI              │
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
- A feature's layers are physically co-located by name (`markdown_tsv`, `diff_checker`, `pdf_checker`) so a new feature is a copy-paste template plus logic.

## 3. Workspace Layout

```text
doctk/
├── Cargo.toml                      # workspace: doctk-core, doctk-cli, tauri app
├── PLAN.md                         # plan index
├── docs/
│   ├── tool-framework.md           # this file
│   ├── feature-markdown-tsv.md
│   ├── feature-diff-checker.md
│   └── feature-pdf-checker.md
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
│   │           ├── markdown_tsv.rs
│   │           ├── diff_checker.rs
│   │           └── pdf_checker.rs
│   └── doctk-cli/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           └── commands/
│               ├── mod.rs
│               ├── markdown_tsv.rs
│               ├── diff_checker.rs
│               └── pdf_checker.rs
└── gui/
    ├── package.json
    ├── src/
    │   ├── main.tsx
    │   ├── App.tsx
    │   ├── shell/
    │   │   ├── ToolLayout.tsx
    │   │   ├── Sidebar.tsx
    │   │   └── ToastProvider.tsx
    │   ├── toolRegistry.tsx
    │   ├── lib/
    │   │   └── tauri.ts
    │   └── features/
    │       ├── markdown-tsv/
    │       │   ├── index.tsx
    │       │   └── components/
    │       │       └── EditableTsvGrid.tsx
    │       ├── diff-checker/
    │       │   ├── index.tsx
    │       │   └── components/
    │       │       ├── SideBySideDiff.tsx
    │       │       └── TrackChangesDiff.tsx
    │       └── pdf-checker/
    │           ├── index.tsx
    │           └── components/
    │               └── PdfResultTable.tsx
    └── src-tauri/
        ├── Cargo.toml
        ├── tauri.conf.json
        ├── capabilities/
        │   └── default.json
        └── src/
            ├── main.rs
            ├── lib.rs
            └── features/
                ├── mod.rs
                ├── markdown_tsv.rs
                ├── diff_checker.rs
                └── pdf_checker.rs
```

## 4. Core Framework: Registry & Feature Contract

### 4.1 Tool manifest

`doctk-core/src/registry.rs` defines the canonical identity of every tool:

```rust
pub struct ToolManifest {
    pub id: &'static str,            // stable machine id, e.g. "markdown_tsv"
    pub name: &'static str,          // display name, e.g. "Markdown ⇄ TSV"
    pub description: &'static str,   // one-line description for tooltips/menus
    pub keywords: &'static [&'static str], // future search/filter
    pub version: &'static str,       // feature version
}

pub const TOOL_REGISTRY: &[ToolManifest] = &[
    markdown_tsv::MANIFEST,
    diff_checker::MANIFEST,
    pdf_checker::MANIFEST,
];
```

Rules:

- `id` is stable and used by CLI subcommands, Tauri command prefixes, and frontend routes. Do not rename after release.
- Core feature modules export a `MANIFEST: ToolManifest` constant.
- `TOOL_REGISTRY` is the single source of truth for "what tools exist" in core. CLI and GUI may wrap this with additional rendering data (icons, routes).

### 4.2 Core feature module contract

Each `doctk-core/src/features/<feature>.rs` must:

- Export a `pub const MANIFEST: ToolManifest`.
- Expose pure, UI-agnostic functions returning `Result<T, DoctkError>`.
- Not depend on `clap`, `tauri`, or any frontend crate.
- Serialize all output models with `serde::{Serialize, Deserialize}`.
- Keep all feature-specific tests in the same file under `#[cfg(test)]` plus fixtures in `crates/doctk-core/tests/fixtures/<feature>/`.

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
├── table        # from feature markdown_tsv
├── diff         # from feature diff_checker
└── pdf          # from feature pdf_checker
```

The shell does:

1. Build top-level `clap::Command` with global flags (`--verbose`, `--json` where applicable).
2. Collect subcommands from `commands::subcommands()`.
3. Match and dispatch to the appropriate feature runner.

### 5.2 Feature command contract

Each `doctk-cli/src/commands/<feature>.rs` must export:

```rust
pub fn cli() -> clap::Command;                          // feature-specific clap definition
pub fn run(matches: &clap::ArgMatches) -> Result<(), DoctkError>; // parse args, call core, format output
```

`commands/mod.rs` aggregates:

```rust
pub fn subcommands() -> Vec<clap::Command> {
    vec![markdown_tsv::cli(), diff_checker::cli(), pdf_checker::cli()]
}

pub fn dispatch(tool_id: &str, matches: &ArgMatches) -> Result<(), DoctkError> {
    match tool_id {
        "markdown_tsv" => markdown_tsv::run(matches),
        "diff_checker" => diff_checker::run(matches),
        "pdf_checker" => pdf_checker::run(matches),
        _ => Err(DoctkError::InvalidArgument(...)),
    }
}
```

Adding a CLI feature = add module + register it in `subcommands()` and `dispatch()`.

### 5.3 Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Processing error (parse, I/O, unsupported input) |
| 2 | CLI usage error (handled by clap) |
| 3 | Tool-specific "check failed" (e.g. PDF page does not fit) |

## 6. GUI Framework

### 6.1 Shell

The React shell renders:

- **Sidebar**: generated from `toolRegistry.tsx`, not hard-coded in the shell.
- **Main area**: renders the active tool's component.
- **Toasts**: global error/success notifications used by all tools.
- **Routing**: simple state-based routing or React Router. The route path is the tool id, e.g. `#/tool/markdown_tsv`.
- **State preservation**: switching between tools must preserve each tool's
  component state. Feature components are kept mounted (e.g. hidden with CSS)
  or their state is lifted to the shell, so switching tabs never resets a tool
  to its initial sample data.

### 6.2 Frontend feature contract

`gui/src/toolRegistry.tsx`:

```ts
export interface ToolDefinition {
  id: string;                 // must match ToolManifest.id in core
  name: string;
  description: string;
  icon: React.ReactNode;
  component: React.LazyExoticComponent<React.ComponentType>;
}

export const TOOL_DEFINITIONS: ToolDefinition[] = [
  { id: 'markdown_tsv', ..., component: lazy(() => import('./features/markdown-tsv')) },
  { id: 'diff_checker', ..., component: lazy(() => import('./features/diff-checker')) },
  { id: 'pdf_checker', ..., component: lazy(() => import('./features/pdf-checker')) },
];
```

Rules:

- Each `gui/src/features/<feature>/index.tsx` default-exports a React component.
- The feature component owns its own state and UI logic; the shell provides only layout and toasts.
- Tool-specific Tauri calls are wrapped in `gui/src/features/<feature>/api.ts` (typed `invoke` wrappers).

### 6.3 Tauri backend feature contract

Each `gui/src-tauri/src/features/<feature>.rs` exposes Tauri command functions:

```rust
#[tauri::command]
pub async fn markdown_to_tsv(md: String) -> Result<String, String> { ... }

#[tauri::command]
pub async fn tsv_to_markdown(tsv: String) -> Result<String, String> { ... }
```

`features/mod.rs` aggregates handlers:

```rust
pub fn handlers() -> impl Fn(tauri::ipc::Invoke) -> bool {
    tauri::generate_handler![
        markdown_tsv::markdown_to_tsv,
        markdown_tsv::tsv_to_markdown,
        diff_checker::diff_side_by_side,
        diff_checker::diff_track_changes,
        pdf_checker::check_pdf_files,
    ]
}
```

`lib.rs` registers `handlers()` and the minimal plugins (`dialog`, `fs` for user-selected files only).

Adding a GUI feature = add frontend folder + Tauri feature module + register in both `mod.rs` files and `toolRegistry.tsx`.

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

impl DoctkError {
    pub fn to_user_message(&self) -> String { ... }
    pub fn exit_code(&self) -> i32 { ... }
}
```

- Core returns typed errors with actionable messages (e.g. `line 5: expected 3 columns, found 4`).
- CLI prints `error: <message>` to stderr and returns the appropriate exit code.
- GUI catches error strings from Tauri commands and shows them as toasts; recoverable validation errors are shown inline by the feature component.

## 8. Logging & Diagnostics

- CLI: `tracing_subscriber` with `RUST_LOG` env var.
- GUI: `tracing` logs to a file in the app-data directory; unhandled errors also surface as toasts.
- Each feature logs its entry points (`tool=markdown_tsv action=md_to_tsv input_bytes=...`).

## 9. Testing Strategy

| Layer | Test type | Tooling |
|---|---|---|
| Core | Unit tests per feature | `cargo test -p doctk-core` |
| Core | Fixture-based integration tests | `tests/fixtures/<feature>/` |
| CLI | End-to-end command tests | `assert_cmd` + `predicates` |
| Tauri backend | Thin-wrapper tests (core is tested) | `cargo test -p doctk-gui` (or equivalent) |
| Frontend | Pure UI logic tests (`gridOps.ts`, diff rendering helpers) | Vitest |
| GUI | Manual cross-platform smoke matrix | Windows/Linux/macOS |

## 10. Build & Distribution

- CLI binaries: `cargo build --release -p doctk-cli` for Linux and macOS.
- GUI bundles: `tauri build` for Windows (NSIS/MSI), Linux (deb/AppImage/rpm), macOS (dmg/app).
- CI matrix:
  - Linux + macOS: CLI build/test.
  - Linux + macOS + Windows: GUI build smoke test.
- Per-feature build isolation is not required in v1; all features ship in one binary/app.

## 11. Dependencies

| Crate / package | Purpose |
|---|---|
| `clap` | CLI parsing |
| `serde` + `serde_json` | Serialization between core, CLI, GUI |
| `thiserror` / `anyhow` | Typed core errors / CLI error propagation |
| `similar` | Line and word diff |
| `lopdf` | PDF parsing |
| `tracing` + `tracing-subscriber` | Logging |
| `tauri` 2.x | GUI shell |
| `@tauri-apps/api` | Frontend `invoke` wrappers |
| `@tauri-apps/plugin-dialog` | Open/save dialogs |
| `@tauri-apps/plugin-fs` | User-selected file read/write |
| Vite + React + TypeScript | Web frontend |
| Vitest | Frontend unit tests |
| Tailwind CSS or CSS modules | Styling (shell + features) |

## 12. Extension Checklist (add a new feature)

1. **Core**
   - [ ] Create `crates/doctk-core/src/features/<feature_id>.rs`.
   - [ ] Define `pub const MANIFEST: ToolManifest`.
   - [ ] Implement pure functions with typed errors and serde models.
   - [ ] Add `mod <feature_id>;` in `features/mod.rs` and add `MANIFEST` to `TOOL_REGISTRY`.
   - [ ] Add unit tests and fixtures.
2. **CLI**
   - [ ] Create `crates/doctk-cli/src/commands/<feature_id>.rs`.
   - [ ] Implement `cli()` and `run()`.
   - [ ] Register in `commands/mod.rs` (`subcommands()` and `dispatch()`).
   - [ ] Add `assert_cmd` tests.
3. **Tauri backend**
   - [ ] Create `gui/src-tauri/src/features/<feature_id>.rs`.
   - [ ] Implement `#[tauri::command]` functions using the core API.
   - [ ] Register commands in `gui/src-tauri/src/features/mod.rs`.
4. **Frontend**
   - [ ] Create `gui/src/features/<feature_id>/index.tsx` and feature-specific components.
   - [ ] Add a `toolRegistry.tsx` entry with `id`, `name`, `description`, `icon`, `component`.
   - [ ] Add Vitest tests for feature-local pure logic.
5. **Release**
   - [ ] Update `PLAN.md` index with a new feature doc.
   - [ ] Add a plan file `docs/feature-<feature_id>.md`.
   - [ ] Update README/help text and verify CLI `--help` and GUI sidebar.

## 13. Extension Scenarios (the framework must support)

- **New converter tool** (e.g. CSV ↔ JSON): follow the checklist; only `doctk-core` feature module and GUI component are substantial.
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
