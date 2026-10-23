import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

import CaseConverterTool from "./case-converter";
import DiffCheckerTool from "./diff-checker";
import MarkdownTextTool from "./markdown-text";
import MarkdownTsvTool from "./markdown-tsv";
import PdfCheckerTool from "./pdf-checker";

// Initial-render tests need neither a native WebView nor a DOM test dependency.
vi.mock("../lib/tauri", () => ({
  convertCase: vi.fn(),
  diffSideBySide: vi.fn(),
  diffTrackChanges: vi.fn(),
  markdownToText: vi.fn(),
  mdToTable: vi.fn(),
  parseTsv: vi.fn(),
  tableToMd: vi.fn(),
  checkPdfFiles: vi.fn(),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-fs", () => ({ readTextFile: vi.fn() }));
vi.mock("@tauri-apps/api/webviewWindow", () => ({ getCurrentWebviewWindow: vi.fn() }));

describe("empty initial document content", () => {
  it.each([
    ["case converter", CaseConverterTool, 2],
    ["diff checker", DiffCheckerTool, 2],
    ["Markdown to text", MarkdownTextTool, 2],
    ["Markdown/TSV", MarkdownTsvTool, 1],
  ] as const)("starts %s textareas empty", (_name, Tool, count) => {
    const html = renderToStaticMarkup(<Tool />);
    const contents = Array.from(
      html.matchAll(/<textarea\b[^>]*>([\s\S]*?)<\/textarea>/g),
      (match) => match[1],
    );
    expect(contents).toHaveLength(count);
    expect(contents.every((value) => value === "")).toBe(true);
  });

  it("keeps the empty TSV grid usable without sample headers or rows", () => {
    const html = renderToStaticMarkup(<MarkdownTsvTool />);
    expect(html).toContain("0 rows × 0 cols");
    expect(html).toContain("<tbody></tbody>");
    expect(html).not.toContain('class="header-cell"');
    expect(html).toContain(">+ Row</button>");
    expect(html).toContain(">+ Column</button>");
    expect(html).toContain(">Paste TSV</button>");
    expect(html).toMatch(/class="grid-scroll"[^>]*tabindex="0"/);
  });

  it("preserves the default diff view", () => {
    const html = renderToStaticMarkup(<DiffCheckerTool />);
    expect(html).toContain('<button class="primary">Side by side</button>');
  });

  it("keeps PDF configuration defaults while starting without files or reports", () => {
    const html = renderToStaticMarkup(<PdfCheckerTool />);
    expect(html).toContain('<option value="a4" selected="">A4</option>');
    expect(html).toContain('<option value="mm" selected="">mm</option>');
    expect(html).toMatch(/<input[^>]*placeholder="Width"[^>]*value="210"/);
    expect(html).toMatch(/<input[^>]*placeholder="Height"[^>]*value="297"/);
    expect(html).toMatch(/<input[^>]*step="0.1"[^>]*value="0.5"/);
    expect(html).toMatch(/<input[^>]*type="checkbox"[^>]*checked=""/);
    expect(html).toContain(">Run check</button>");
    expect(html).not.toContain('class="file-path"');
    expect(html).not.toContain("<tbody>");
  });
});
