export interface GridState {
  headers: string[];
  rows: string[][];
}

export function addRow(grid: GridState, atIndex?: number): GridState {
  const width = grid.headers.length;
  const newRow = Array.from({ length: width }, () => "");
  const rows = grid.rows.slice();
  rows.splice(atIndex ?? rows.length, 0, newRow);
  return { ...grid, rows };
}

export function deleteRow(grid: GridState, index: number): GridState {
  const rows = grid.rows.slice();
  rows.splice(index, 1);
  return { ...grid, rows };
}

export function addColumn(grid: GridState, atIndex?: number): GridState {
  const width = grid.headers.length;
  const col = atIndex ?? width;
  const headers = grid.headers.slice();
  headers.splice(col, 0, `col_${width + 1}`);
  const rows = grid.rows.map((row) => {
    const next = row.slice();
    next.splice(col, 0, "");
    return next;
  });
  return { headers, rows };
}

export function deleteColumn(grid: GridState, index: number): GridState {
  const headers = grid.headers.slice();
  headers.splice(index, 1);
  const rows = grid.rows.map((row) => {
    const next = row.slice();
    next.splice(index, 1);
    return next;
  });
  return { headers, rows };
}

export function updateCell(
  grid: GridState,
  rowIndex: number,
  colIndex: number,
  value: string,
): GridState {
  const rows = grid.rows.map((row) => row.slice());
  if (rowIndex >= 0 && rowIndex < rows.length && colIndex >= 0 && colIndex < rows[rowIndex].length) {
    rows[rowIndex][colIndex] = value;
  }
  return { ...grid, rows };
}

export function updateHeader(
  grid: GridState,
  colIndex: number,
  value: string,
): GridState {
  const headers = grid.headers.slice();
  if (colIndex >= 0 && colIndex < headers.length) {
    headers[colIndex] = value;
  }
  return { ...grid, headers };
}

export function sanitizeCell(value: string): string {
  return value.replace(/\t/g, " ").replace(/\r?\n/g, " ");
}

/**
 * A rectangular selection over the grid.
 *
 * `r1`/`r2` are inclusive body-row indexes.  A value of `-1` means the header
 * row, so a column-header selection is represented as `{ r1: -1, r2: -1, ... }`.
 */
export interface GridSelection {
  r1: number;
  r2: number;
  c1: number;
  c2: number;
}

export function clampSelection(
  selection: GridSelection,
  grid: GridState,
): GridSelection {
  const lastBodyRow = grid.rows.length - 1;
  const lastCol = Math.max(0, grid.headers.length - 1);
  const clampRow = (value: number) => Math.max(-1, Math.min(lastBodyRow, value));
  const clampCol = (value: number) => Math.max(0, Math.min(lastCol, value));
  const r1 = clampRow(Math.min(selection.r1, selection.r2));
  const r2 = clampRow(Math.max(selection.r1, selection.r2));
  const c1 = clampCol(Math.min(selection.c1, selection.c2));
  const c2 = clampCol(Math.max(selection.c1, selection.c2));
  return { r1, r2, c1, c2 };
}

function selectedCellText(
  grid: GridState,
  rowIndex: number,
  colIndex: number,
): string {
  if (rowIndex === -1) {
    return sanitizeCell(grid.headers[colIndex] ?? "");
  }
  const row = grid.rows[rowIndex];
  if (!row) return "";
  return sanitizeCell(row[colIndex] ?? "");
}

/**
 * Serialize the selected range (including the header row when the selection
 * includes row index `-1`) as plain TSV.
 */
export function selectionToTsv(
  grid: GridState,
  selection: GridSelection,
): string {
  const sel = clampSelection(selection, grid);
  const lines: string[] = [];
  for (let rowIndex = sel.r1; rowIndex <= sel.r2; rowIndex += 1) {
    const cells: string[] = [];
    for (let colIndex = sel.c1; colIndex <= sel.c2; colIndex += 1) {
      cells.push(selectedCellText(grid, rowIndex, colIndex));
    }
    lines.push(cells.join("\t"));
  }
  return lines.join("\n");
}

/** Serialize the whole table (header + body) as plain TSV. */
export function tableToTsv(grid: GridState): string {
  const lines: string[] = [];
  lines.push(grid.headers.map((header) => sanitizeCell(header)).join("\t"));
  for (const row of grid.rows) {
    lines.push(row.map((cell) => sanitizeCell(cell)).join("\t"));
  }
  return lines.join("\n");
}
