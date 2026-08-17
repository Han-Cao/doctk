import { describe, expect, it } from "vitest";
import {
  addColumn,
  addRow,
  deleteColumn,
  deleteRow,
  sanitizeCell,
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
});
