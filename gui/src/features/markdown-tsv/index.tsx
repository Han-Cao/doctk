import { useState } from "react";
import { mdToTable, tableToMd, type Table } from "../../lib/tauri";
import { addColumn, addRow, deleteColumn, deleteRow, updateCell, updateHeader, sanitizeCell } from "./gridOps";

const SAMPLE_MD = "| name | qty |\n| --- | ---: |\n| apple | 4 |\n| pear | 7 |\n";

export default function MarkdownTsvTool() {
  const [markdown, setMarkdown] = useState(SAMPLE_MD);
  const [table, setTable] = useState<Table | null>({ headers: ["name", "qty"], rows: [["apple", "4"], ["pear", "7"]] });
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

  function mutate(mutator: (t: Table) => Table) {
    if (!table) return;
    setTable(mutator(table));
    setDirty(true);
  }

  return (
    <div className="pane-grid">
      <div className="pane">
        <label>Markdown table (plain text)</label>
        <textarea
          rows={16}
          value={markdown}
          onChange={(event) => {
            setMarkdown(event.target.value);
            setDirty(true);
          }}
          spellCheck={false}
        />
        <div className="toolbar">
          <button className="primary" onClick={convertMdToTable}>Convert to Table →</button>
        </div>
        <p className="hint">Edit markdown, then click to fill the editable TSV table.</p>
      </div>

      <div className="pane">
        <label>TSV table viewer {dirty ? "(edited)" : ""}</label>
        {table ? (
          <EditableGrid
            table={table}
            onChange={(next) => {
              setTable(next);
              setDirty(true);
            }}
            onAddRow={() => mutate((t) => addRow(t))}
            onAddColumn={() => mutate((t) => addColumn(t))}
          />
        ) : (
          <p className="hint">No table loaded.</p>
        )}
        <div className="toolbar">
          <button className="primary" onClick={convertTableToMd}>← Convert to Markdown</button>
        </div>
        <p className="hint">Double-click a cell to edit. Use row/column ✕ buttons to delete; toolbar buttons add rows/columns.</p>
      </div>

      {error && <div className="error" style={{ gridColumn: "1 / -1" }}>{error}</div>}
    </div>
  );
}

function EditableGrid(props: {
  table: Table;
  onChange: (next: Table) => void;
  onAddRow: () => void;
  onAddColumn: () => void;
}) {
  const { table, onChange } = props;
  const width = table.headers.length;

  return (
    <div style={{ overflow: "auto", maxHeight: 420 }}>
      <table className="grid-table">
        <thead>
          <tr>
            <th style={{ width: 36 }}></th>
            {table.headers.map((header, colIndex) => (
              <th key={`h-${colIndex}`}>
                <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                  <input
                    value={header}
                    onChange={(event) => onChange(updateHeader(table, colIndex, sanitizeCell(event.target.value)))}
                  />
                  <button
                    className="danger"
                    title="Delete column"
                    onClick={() => onChange(deleteColumn(table, colIndex))}
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
              <td>
                <button
                  className="danger"
                  title="Delete row"
                  onClick={() => onChange(deleteRow(table, rowIndex))}
                >
                  ✕
                </button>
              </td>
              {row.map((cell, colIndex) => (
                <td key={`c-${rowIndex}-${colIndex}`}>
                  <input
                    value={cell}
                    onChange={(event) =>
                      onChange(updateCell(table, rowIndex, colIndex, sanitizeCell(event.target.value)))
                    }
                  />
                </td>
              ))}
              {Array.from({ length: Math.max(0, width - row.length) }, (_, i) => (
                <td key={`pad-${rowIndex}-${i}`}>
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
      <div className="toolbar" style={{ marginTop: 8 }}>
        <button onClick={props.onAddRow}>+ Row</button>
        <button onClick={props.onAddColumn}>+ Column</button>
      </div>
    </div>
  );
}
