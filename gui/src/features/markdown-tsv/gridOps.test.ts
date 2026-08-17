import { describe, expect, it } from "vitest";
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
} from "./gridOps";

const base = {
  headers: ["a", "b"],
  rows: [
    ["1", "2"],
    ["3", "4"],
  ],
};

describe("gridOps", () => {
  it("adds a row at the end", () => {
    const next = addRow(base);
    expect(next.rows).toHaveLength(3);
    expect(next.rows[2]).toEqual(["", ""]);
  });

  it("deletes a row", () => {
    const next = deleteRow(base, 0);
    expect(next.rows).toEqual([["3", "4"]]);
  });

  it("adds a column", () => {
    const next = addColumn(base);
    expect(next.headers).toEqual(["a", "b", "col_3"]);
    expect(next.rows[0]).toEqual(["1", "2", ""]);
  });

  it("deletes a column", () => {
    const next = deleteColumn(base, 0);
    expect(next.headers).toEqual(["b"]);
    expect(next.rows[0]).toEqual(["2"]);
  });

  it("updates a cell", () => {
    const next = updateCell(base, 0, 1, "x");
    expect(next.rows[0][1]).toBe("x");
    expect(base.rows[0][1]).toBe("2");
  });

  it("sanitizes tabs and newlines", () => {
    expect(sanitizeCell("a\tb\nc")).toBe("a b c");
  });

  it("serializes the whole table to TSV", () => {
    expect(tableToTsv(base)).toBe("a\tb\n1\t2\n3\t4");
  });

  it("serializes a cell selection to TSV", () => {
    const tsv = selectionToTsv(base, { r1: 0, r2: 0, c1: 0, c2: 1 });
    expect(tsv).toBe("1\t2");
  });

  it("serializes a column selection including the header", () => {
    const tsv = selectionToTsv(base, { r1: -1, r2: 1, c1: 0, c2: 0 });
    expect(tsv).toBe("a\n1\n3");
  });

  it("clamps selection to the grid bounds", () => {
    const sel = clampSelection({ r1: -5, r2: 99, c1: -1, c2: 5 }, base);
    expect(sel).toEqual({ r1: -1, r2: 1, c1: 0, c2: 1 });
  });
});
