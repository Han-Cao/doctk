import { useState } from "react";
import { mdToTable, parseTsv, tableToMd, type Table } from "../../lib/tauri";
import {
  addColumn,
  addRow,
  clampSelection,
  deleteColumn,
  deleteRow,
  sanitizeCell,
  selectionToTsv,
  tableToTsv,
  updateCell,
  updateHeader,
  type GridSelection,
} from "./gridOps";

const SAMPLE_MD =
  "| name | qty | price |\n| --- | ---: | ---: |\n| apple | 4 | 1.20 |\n| pear | 7 | 0.80 |\n| plum | 5 | 2.10 |\n| kiwi | 3 | 3.40 |\n| fig | 6 | 1.75 |\n";

export default function MarkdownTsvTool() {
  const [markdown, setMarkdown] = useState(SAMPLE_MD);
  const [table, setTable] = useState<Table | null>({
    headers: ["name", "qty", "price"],
    rows: [
      ["apple", "4", "1.20"],
      ["pear", "7", "0.80"],
      ["plum", "5", "2.10"],
      ["kiwi", "3", "3.40"],
      ["fig", "6", "1.75"],
    ],
  });
  const [error, setError] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);

  async function convertMdToTable() {
    try {
      const parsed = await mdToTable(markdown);
      setTable(parsed);
      setDirty(false);
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }

  async function convertTableToMd() {
    if (!table) return;
    try {
      const md = await tableToMd(table);
      setMarkdown(md);
      setDirty(false);
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }

  const markdownLines = markdown.split("\n").length;

  return (
    <div className="stack-vertical">
      <section className="tool-box markdown-box">
        <div className="tool-box-header">
          <label>Markdown table (plain text)</label>
        </div>
        <textarea
          className="markdown-editor"
          value={markdown}
          onChange={(event) => {
            setMarkdown(event.target.value);
            setDirty(true);
          }}
          spellCheck={false}
        />
        <div className="status-bar">
          {markdownLines} line{markdownLines === 1 ? "" : "s"} · {markdown.length} characters
        </div>
        <div className="toolbar conversion-toolbar">
          <button className="primary" onClick={convertMdToTable}>
            Convert to Table ↓
          </button>
          <button className="primary" onClick={convertTableToMd}>
            Convert to Markdown ↑
          </button>
        </div>
      </section>

      <section className="tool-box tsv-box">
        <div className="tool-box-header">
          <label>TSV table viewer {dirty ? "(edited)" : ""}</label>
        </div>
        {table ? (
          <EditableGrid
            table={table}
            onChange={(next) => {
              setTable(next);
              setDirty(true);
            }}
          />
        ) : (
          <p className="hint">No table loaded.</p>
        )}
        <div className="status-bar">
          {table ? `${table.rows.length} rows × ${table.headers.length} cols` : "0 rows × 0 cols"}
        </div>
      </section>

      {error && <div className="error">{error}</div>}
    </div>
  );
}

function EditableGrid(props: {
  table: Table;
  onChange: (next: Table) => void;
}) {
  const { table, onChange } = props;
  const [selection, setSelection] = useState<GridSelection | null>(null);
  const [clipboardMessage, setClipboardMessage] = useState<string | null>(null);
  const width = table.headers.length;
  const lastCol = Math.max(0, width - 1);
  const lastBodyRow = table.rows.length - 1;

  function isColumnSelection(sel: GridSelection): boolean {
    return sel.r1 === -1 && sel.r2 === lastBodyRow;
  }

  function isRowSelection(sel: GridSelection): boolean {
    return sel.c1 === 0 && sel.c2 === lastCol;
  }

  function handleCellClick(rowIndex: number, colIndex: number) {
    // Shift+click on cells does not create cell ranges; it selects the clicked cell.
    setSelection({ r1: rowIndex, r2: rowIndex, c1: colIndex, c2: colIndex });
  }

  function handleColumnHeaderClick(colIndex: number, shiftKey: boolean) {
    setSelection((previous) => {
      if (shiftKey && previous && isColumnSelection(previous)) {
        return {
          r1: -1,
          r2: lastBodyRow,
          c1: Math.min(previous.c1, colIndex),
          c2: Math.max(previous.c2, colIndex),
        };
      }
      return { r1: -1, r2: lastBodyRow, c1: colIndex, c2: colIndex };
    });
  }

  function handleRowHeaderClick(rowIndex: number, shiftKey: boolean) {
    setSelection((previous) => {
      if (shiftKey && previous && isRowSelection(previous)) {
        return {
          r1: Math.min(previous.r1, rowIndex),
          r2: Math.max(previous.r2, rowIndex),
          c1: 0,
          c2: lastCol,
        };
      }
      return { r1: rowIndex, r2: rowIndex, c1: 0, c2: lastCol };
    });
  }

  function writeClipboard(tsv: string, successMessage: string) {
    const clipboard = navigator.clipboard;
    if (!clipboard) {
      setClipboardMessage("Clipboard API is not available in this WebView.");
      return;
    }
    clipboard.writeText(tsv).then(
      () => setClipboardMessage(successMessage),
      (err) => setClipboardMessage(`Copy failed: ${String(err)}`),
    );
  }

  function copyAsTsv() {
    writeClipboard(tableToTsv(table), "Copied whole table as TSV.");
  }

  function copySelectionAsTsv() {
    if (!selection) {
      copyAsTsv();
      return;
    }
    writeClipboard(selectionToTsv(table, selection), "Copied selected cells as TSV.");
  }

  async function pasteTsvFromClipboard(text: string) {
    if (!text.trim()) return;
    try {
      const parsed = await parseTsv(text);
      onChange(parsed);
      setSelection(null);
      setClipboardMessage("Pasted TSV into table.");
    } catch (err) {
      setClipboardMessage(`Paste failed: ${String(err)}`);
    }
  }

  function handleCopy(event: React.ClipboardEvent<HTMLDivElement>) {
    if (!selection) {
      event.preventDefault();
      event.clipboardData.setData("text/plain", tableToTsv(table));
      setClipboardMessage("Copied whole table as TSV.");
      return;
    }
    event.preventDefault();
    event.clipboardData.setData("text/plain", selectionToTsv(table, selection));
    setClipboardMessage("Copied selected cells as TSV.");
  }

  function handlePaste(event: React.ClipboardEvent<HTMLDivElement>) {
    event.preventDefault();
    void pasteTsvFromClipboard(event.clipboardData.getData("text/plain"));
  }

  function isSelected(rowIndex: number, colIndex: number): boolean {
    if (!selection) return false;
    const sel = clampSelection(selection, table);
    return rowIndex >= sel.r1 && rowIndex <= sel.r2 && colIndex >= sel.c1 && colIndex <= sel.c2;
  }

  return (
    <div className="grid-shell">
      <div className="toolbar grid-toolbar">
        <button onClick={() => onChange(addRow(table))}>+ Row</button>
        <button onClick={() => onChange(addColumn(table))}>+ Column</button>
        <button onClick={copySelectionAsTsv}>Copy TSV</button>
        <button
          onClick={async () => {
            try {
              if (!navigator.clipboard) {
                setClipboardMessage("Clipboard API is not available in this WebView.");
                return;
              }
              const text = await navigator.clipboard.readText();
              await pasteTsvFromClipboard(text);
            } catch (err) {
              setClipboardMessage(`Paste failed: ${String(err)}`);
            }
          }}
        >
          Paste TSV
        </button>
      </div>

      <div className="grid-scroll" onCopy={handleCopy} onPaste={handlePaste}>
        <table className="grid-table">
          <thead>
            <tr>
              <th
                className="corner-cell"
                title="Select whole table"
                onClick={() =>
                  setSelection({ r1: -1, r2: lastBodyRow, c1: 0, c2: lastCol })
                }
              >
                ⬚
              </th>
              {table.headers.map((header, colIndex) => (
                <th
                  key={`h-${colIndex}`}
                  className={`column-header ${isSelected(-1, colIndex) ? "selected" : ""}`}
                  onClick={(event) => handleColumnHeaderClick(colIndex, event.shiftKey)}
                >
                  <div className="header-cell">
                    <input
                      value={header}
                      onClick={(event) => event.stopPropagation()}
                      onChange={(event) =>
                        onChange(updateHeader(table, colIndex, sanitizeCell(event.target.value)))
                      }
                    />
                    <button
                      className="danger"
                      title="Delete column"
                      onClick={(event) => {
                        event.stopPropagation();
                        onChange(deleteColumn(table, colIndex));
                      }}
                    >
                      ✕
                    </button>
                  </div>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {table.rows.map((row, rowIndex) => (
              <tr key={`r-${rowIndex}`}>
                <th
                  className={`row-header ${isSelected(rowIndex, -1) ? "selected" : ""}`}
                  onClick={(event) => handleRowHeaderClick(rowIndex, event.shiftKey)}
                >
                  <div className="row-header-cell">
                    <span>{rowIndex + 1}</span>
                    <button
                      className="danger"
                      title="Delete row"
                      onClick={(event) => {
                        event.stopPropagation();
                        onChange(deleteRow(table, rowIndex));
                      }}
                    >
                      ✕
                    </button>
                  </div>
                </th>
                {row.map((cell, colIndex) => (
                  <td
                    key={`c-${rowIndex}-${colIndex}`}
                    className={isSelected(rowIndex, colIndex) ? "selected" : ""}
                    onClick={() => handleCellClick(rowIndex, colIndex)}
                  >
                    <input
                      value={cell}
                      onChange={(event) =>
                        onChange(updateCell(table, rowIndex, colIndex, sanitizeCell(event.target.value)))
                      }
                    />
                  </td>
                ))}
                {Array.from({ length: Math.max(0, width - row.length) }, (_, i) => (
                  <td
                    key={`pad-${rowIndex}-${i}`}
                    className={isSelected(rowIndex, row.length + i) ? "selected" : ""}
                    onClick={() => handleCellClick(rowIndex, row.length + i)}
                  >
                    <input
                      value=""
                      onChange={(event) =>
                        onChange(updateCell(table, rowIndex, row.length + i, sanitizeCell(event.target.value)))
                      }
                    />
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {clipboardMessage && <p className="hint clipboard-message">{clipboardMessage}</p>}
    </div>
  );
}
