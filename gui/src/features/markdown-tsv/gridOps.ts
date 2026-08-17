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
