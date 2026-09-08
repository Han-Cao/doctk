import type { CaseMode } from "../../lib/tauri";

export interface CaseModeDefinition {
  mode: CaseMode;
  label: string;
  description: string;
}

export const CASE_MODES: CaseModeDefinition[] = [
  {
    mode: "sentence",
    label: "Sentence case",
    description:
      "Capitalizes the first word of each sentence and any listed proper nouns.",
  },
  {
    mode: "lower",
    label: "lower case",
    description: "Converts every letter to lowercase.",
  },
  {
    mode: "upper",
    label: "UPPER CASE",
    description: "Converts every letter to uppercase.",
  },
  {
    mode: "capitalized",
    label: "Capitalized Case",
    description: "Capitalizes the first letter of every word.",
  },
  {
    mode: "title",
    label: "Title Case",
    description:
      "Capitalizes major and long words; keeps minor words lowercase unless they begin or end a title; applies listed proper nouns.",
  },
];

export function showProperNouns(mode: CaseMode | null): boolean {
  return mode === "sentence" || mode === "title";
}

export function parseProperNounInput(input: string): string[] {
  return input
    .split(/\r\n|\r|\n/)
    .map((entry) => entry.trim())
    .filter((entry) => entry.length > 0);
}
