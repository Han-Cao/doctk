# Document Processing Tool — Plan Index

This repo contains the implementation plan for the GUI & CLI document processing tool.

## Plan documents

| Document | Purpose |
|---|---|
| [`tool-framework.md`](tool-framework.md) | Overall tool framework design plan (architecture, feature registry, layer contracts, extension guide) |
| [`feature-diff-checker.md`](feature-diff-checker.md) | Feature plan: Diff checker |
| [`feature-markdown-tsv.md`](feature-markdown-tsv.md) | Feature plan: Markdown table ↔ TSV conversion |
| [`feature-markdown-text.md`](feature-markdown-text.md) | Feature plan: Markdown → Text |
| [`feature-pdf-checker.md`](feature-pdf-checker.md) | Feature plan: PDF checker |

## Quick start

1. Read `tool-framework.md` first — it defines how features plug into the GUI and CLI.
2. Read the individual feature docs for concrete data models, UI behavior, and test plans.
3. To add a new feature, follow the checklist at the bottom of `tool-framework.md`.
