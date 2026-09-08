import { invoke } from "@tauri-apps/api/core";

export interface Table {
  headers: string[];
  rows: string[][];
}

export type Alignment = "Default" | "Left" | "Center" | "Right";

export interface MarkdownTable {
  table: Table;
  alignments: Alignment[];
}

export type LineKind = "Equal" | "Delete" | "Insert" | "Replace";

export interface WordRange {
  left_start: number;
  left_len: number;
  right_start: number;
  right_len: number;
}

export interface SideBySideLine {
  line_number_left: number | null;
  line_number_right: number | null;
  left_text: string;
  right_text: string;
  kind: LineKind;
  word_diff: WordRange[];
}

export interface SideBySideDiff {
  lines: SideBySideLine[];
}

export type TrackChangeKind = "Equal" | "Inserted" | "Deleted";

export interface TrackChangeSegment {
  text: string;
  kind: TrackChangeKind;
}

export interface TrackChangesDiff {
  segments: TrackChangeSegment[];
}

export type CaseMode = "sentence" | "lower" | "upper" | "capitalized" | "title";

export interface PaperSize {
  name: string;
  width_pt: number;
  height_pt: number;
}

export type ColorMode = "Unknown" | "Gray" | "Rgb" | "Cmyk" | "Mixed";

export interface PdfPageReport {
  page_number: number;
  width_pt: number;
  height_pt: number;
  color_mode: ColorMode;
  color_spaces: string[];
  fits: boolean;
  fit_reason: string | null;
}

export interface PdfFileReport {
  path: string;
  is_encrypted: boolean;
  pages: PdfPageReport[];
  error: string | null;
}

export async function mdToTable(md: string): Promise<Table> {
  return invoke<Table>("markdown_tsv_md_to_table", { md });
}

export async function parseTsv(tsv: string): Promise<Table> {
  return invoke<Table>("markdown_tsv_parse_tsv", { tsv });
}

export async function tableToMd(table: Table): Promise<string> {
  return invoke<string>("markdown_tsv_table_to_md", { table });
}

export async function markdownToText(md: string): Promise<string> {
  return invoke<string>("markdown_text_convert", { md });
}

export async function convertCase(
  text: string,
  mode: CaseMode,
  properNouns: string[],
): Promise<string> {
  return invoke<string>("case_converter_convert", { text, mode, properNouns });
}

export async function diffSideBySide(left: string, right: string): Promise<SideBySideDiff> {
  return invoke<SideBySideDiff>("diff_checker_side_by_side", { left, right });
}

export async function diffTrackChanges(left: string, right: string): Promise<TrackChangesDiff> {
  return invoke<TrackChangesDiff>("diff_checker_track_changes", { left, right });
}

export async function checkPdfFiles(
  paths: string[],
  paper: PaperSize,
  tolerancePt: number,
  ignoreOrientation: boolean,
): Promise<PdfFileReport[]> {
  return invoke<PdfFileReport[]>("pdf_checker_check_files", {
    paths,
    paper,
    tolerancePt,
    ignoreOrientation,
  });
}
