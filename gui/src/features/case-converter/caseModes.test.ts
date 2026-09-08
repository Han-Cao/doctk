import { describe, expect, it } from "vitest";

import { CASE_MODES, parseProperNounInput, showProperNouns } from "./caseModes";

describe("parseProperNounInput", () => {
  it("splits on newlines, trims entries, and drops blanks", () => {
    expect(parseProperNounInput("John\n\n New York \nNASA")).toEqual([
      "John",
      "New York",
      "NASA",
    ]);
  });

  it("handles CRLF and lone CR line endings", () => {
    expect(parseProperNounInput("John\r\nNew York\rNASA")).toEqual([
      "John",
      "New York",
      "NASA",
    ]);
  });

  it("returns an empty list for empty or whitespace-only input", () => {
    expect(parseProperNounInput("")).toEqual([]);
    expect(parseProperNounInput("  \n\t\n")).toEqual([]);
  });
});

describe("CASE_MODES", () => {
  it("lists all five modes in display order", () => {
    expect(CASE_MODES.map((definition) => definition.mode)).toEqual([
      "sentence",
      "lower",
      "upper",
      "capitalized",
      "title",
    ]);
  });

  it("provides a label and description for every mode", () => {
    for (const definition of CASE_MODES) {
      expect(definition.label.length).toBeGreaterThan(0);
      expect(definition.description.length).toBeGreaterThan(0);
    }
  });
});

describe("showProperNouns", () => {
  it("shows the proper-noun section for sentence and title modes only", () => {
    expect(showProperNouns("sentence")).toBe(true);
    expect(showProperNouns("title")).toBe(true);
    expect(showProperNouns("lower")).toBe(false);
    expect(showProperNouns("upper")).toBe(false);
    expect(showProperNouns("capitalized")).toBe(false);
    expect(showProperNouns(null)).toBe(false);
  });
});
