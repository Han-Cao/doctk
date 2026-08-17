import { lazy } from "react";

export interface ToolDefinition {
  id: string;
  name: string;
  description: string;
  icon: string;
  component: React.LazyExoticComponent<React.ComponentType>;
}

export const TOOL_DEFINITIONS: ToolDefinition[] = [
  {
    id: "markdown_tsv",
    name: "Markdown ⇄ TSV",
    description: "Convert between markdown tables and TSV, with an editable table viewer.",
    icon: "🔁",
    component: lazy(() => import("./features/markdown-tsv")),
  },
  {
    id: "diff_checker",
    name: "Diff Checker",
    description: "Side-by-side diff with word highlights and Word-style track changes.",
    icon: "🔍",
    component: lazy(() => import("./features/diff-checker")),
  },
  {
    id: "pdf_checker",
    name: "PDF Checker",
    description: "Check PDF/AI page size and color mode against paper-size presets.",
    icon: "📄",
    component: lazy(() => import("./features/pdf-checker")),
  },
];
