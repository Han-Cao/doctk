# Document Processing Tool — Plan Index

This repo contains the implementation plan for the GUI & CLI document processing tool.

## Plan documents

| Document | Purpose |
|---|---|
| [`docs/tool-framework.md`](docs/tool-framework.md) | Overall tool framework design plan (architecture, feature registry, layer contracts, extension guide) |
| [`docs/feature-markdown-tsv.md`](docs/feature-markdown-tsv.md) | Feature plan: Markdown table ↔ TSV conversion |
| [`docs/feature-diff-checker.md`](docs/feature-diff-checker.md) | Feature plan: Diff checker |
| [`docs/feature-pdf-checker.md`](docs/feature-pdf-checker.md) | Feature plan: PDF checker |

## Quick start

1. Read `docs/tool-framework.md` first — it defines how features plug into the GUI and CLI.
2. Read the individual feature docs for concrete data models, UI behavior, and test plans.
3. To add a new feature, follow the checklist at the bottom of `docs/tool-framework.md`.
