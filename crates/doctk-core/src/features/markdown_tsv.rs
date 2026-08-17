//! Markdown table <-> TSV conversion feature.
//!
//! Implements the core logic behind the "Markdown ⇄ TSV" tool.  The GUI uses
//! the structured [`Table`] and [`MarkdownTable`] types; the CLI uses the
//! string-level convenience helpers.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::registry::ToolManifest;

pub const MANIFEST: ToolManifest = ToolManifest {
    id: "markdown_tsv",
    name: "Markdown ⇄ TSV",
    description: "Convert between markdown tables and TSV, with an editable table viewer.",
    keywords: &["markdown", "tsv", "table", "convert", "spreadsheet"],
    version: "0.1.0",
};

/// A plain grid: headers plus body rows.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Table {
    pub fn is_empty(&self) -> bool {
        self.headers.is_empty() && self.rows.is_empty()
    }

    pub fn column_count(&self) -> usize {
        self.headers.len()
    }
}

/// Column alignment for markdown output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Alignment {
    #[default]
    Default,
    Left,
    Center,
    Right,
}

/// A markdown table with alignment metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownTable {
    pub table: Table,
    pub alignments: Vec<Alignment>,
}

/// Errors produced by markdown/TSV parsing and rendering.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TableError {
    #[error("no table found: input is empty")]
    EmptyInput,

    #[error("line {line}: expected a markdown delimiter row such as `|---|---|`")]
    ExpectedDelimiter { line: usize },

    #[error("line {line}: invalid delimiter cell `{cell}`")]
    InvalidDelimiter { line: usize, cell: String },

    #[error("line {line}: expected {expected} columns, found {actual}")]
    InconsistentColumns {
        line: usize,
        expected: usize,
        actual: usize,
    },
}

fn normalize_line_endings(text: &str) -> std::borrow::Cow<'_, str> {
    if text.contains("\r\n") {
        std::borrow::Cow::Owned(text.replace("\r\n", "\n"))
    } else if text.contains('\r') {
        std::borrow::Cow::Owned(text.replace('\r', "\n"))
    } else {
        std::borrow::Cow::Borrowed(text)
    }
}

/// Split a row on unescaped `|`.  Backslash escapes are preserved and later
/// converted by [`unescape_cell`].
fn split_unescaped(s: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut escaped = false;

    for ch in s.chars() {
        if escaped {
            current.push('\\');
            current.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '|' {
            cells.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    if escaped {
        current.push('\\');
    }
    cells.push(current);
    cells
}

fn unescape_cell(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('|') => out.push('|'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Parse one markdown table row into cells.
fn split_row(line: &str) -> Vec<String> {
    let line = line.trim();
    let line = line.strip_prefix('|').unwrap_or(line);
    let line = line.strip_suffix('|').unwrap_or(line);
    split_unescaped(line)
        .into_iter()
        .map(|cell| unescape_cell(cell.trim()))
        .collect()
}

fn parse_alignment(cell: &str) -> Option<Alignment> {
    let mut s = cell.trim().to_string();
    let left = s.starts_with(':');
    let right = s.ends_with(':');
    if left {
        s.remove(0);
    }
    if right {
        s.pop();
    }
    if s.len() < 3 || !s.chars().all(|c| c == '-') {
        return None;
    }
    Some(match (left, right) {
        (true, true) => Alignment::Center,
        (true, false) => Alignment::Left,
        (false, true) => Alignment::Right,
        (false, false) => Alignment::Default,
    })
}

/// Parse a GitHub-style markdown table from `text`.
pub fn parse_markdown_table(text: &str) -> Result<MarkdownTable, TableError> {
    let text = normalize_line_endings(text);
    let lines: Vec<&str> = text.split('\n').collect();

    let first = lines
        .iter()
        .position(|line| !line.trim().is_empty())
        .ok_or(TableError::EmptyInput)?;
    let header_line = first;
    let delimiter_line = header_line + 1;
    if delimiter_line >= lines.len() || lines[delimiter_line].trim().is_empty() {
        return Err(TableError::ExpectedDelimiter {
            line: delimiter_line + 1,
        });
    }

    let headers = split_row(lines[header_line]);
    let expected = headers.len();
    if expected == 0 {
        return Err(TableError::ExpectedDelimiter {
            line: delimiter_line + 1,
        });
    }

    let delimiter_cells = split_row(lines[delimiter_line]);
    if delimiter_cells.len() != expected {
        return Err(TableError::InconsistentColumns {
            line: delimiter_line + 1,
            expected,
            actual: delimiter_cells.len(),
        });
    }

    let mut alignments = Vec::with_capacity(expected);
    for cell in &delimiter_cells {
        match parse_alignment(cell) {
            Some(alignment) => alignments.push(alignment),
            None => {
                return Err(TableError::InvalidDelimiter {
                    line: delimiter_line + 1,
                    cell: cell.clone(),
                });
            }
        }
    }

    let mut rows = Vec::new();
    for (idx, line) in lines[delimiter_line + 1..].iter().enumerate() {
        let line_number = delimiter_line + 2 + idx; // 1-based
        if line.trim().is_empty() {
            continue; // blank lines separate the table from surrounding text
        }
        let cells = split_row(line);
        if cells.len() != expected {
            return Err(TableError::InconsistentColumns {
                line: line_number,
                expected,
                actual: cells.len(),
            });
        }
        rows.push(cells);
    }

    Ok(MarkdownTable {
        table: Table { headers, rows },
        alignments,
    })
}

fn escape_markdown_cell(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', "<br>")
        .replace('\t', " ")
}

fn alignment_marker(alignment: Alignment) -> &'static str {
    match alignment {
        Alignment::Default => "---",
        Alignment::Left => ":---",
        Alignment::Center => ":---:",
        Alignment::Right => "---:",
    }
}

/// Render a [`MarkdownTable`] as a GitHub-style markdown table.
pub fn markdown_table_to_string(md: &MarkdownTable) -> String {
    if md.table.headers.is_empty() {
        return String::new();
    }

    let mut lines = Vec::with_capacity(md.table.rows.len() + 2);
    lines.push(format!(
        "| {} |",
        md.table
            .headers
            .iter()
            .map(|h| escape_markdown_cell(h))
            .collect::<Vec<_>>()
            .join(" | ")
    ));
    lines.push(format!(
        "| {} |",
        md.alignments
            .iter()
            .map(|a| alignment_marker(*a).to_string())
            .collect::<Vec<_>>()
            .join(" | ")
    ));
    for row in &md.table.rows {
        lines.push(format!(
            "| {} |",
            row.iter()
                .map(|cell| escape_markdown_cell(cell))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
    }
    lines.join("\n")
}

/// Parse plain TSV text into a [`Table`].
pub fn parse_tsv(text: &str) -> Result<Table, TableError> {
    let text = normalize_line_endings(text);
    let mut lines: Vec<&str> = text.split('\n').collect();

    // A trailing newline is a line terminator, not an empty row.
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    if lines.is_empty() {
        return Ok(Table::default());
    }

    let mut rows = Vec::with_capacity(lines.len());
    for (idx, line) in lines.iter().enumerate() {
        let cells: Vec<String> = line.split('\t').map(str::to_string).collect();
        rows.push((idx + 1, cells));
    }

    let expected = rows[0].1.len();
    let mut table = Table {
        headers: rows[0].1.clone(),
        rows: Vec::with_capacity(rows.len() - 1),
    };
    for (line_number, cells) in rows.into_iter().skip(1) {
        if cells.len() != expected {
            return Err(TableError::InconsistentColumns {
                line: line_number,
                expected,
                actual: cells.len(),
            });
        }
        table.rows.push(cells);
    }
    Ok(table)
}

/// Render a [`Table`] as plain TSV text.
pub fn tsv_to_string(table: &Table) -> String {
    if table.headers.is_empty() {
        return String::new();
    }
    let mut lines = Vec::with_capacity(table.rows.len() + 1);
    lines.push(table.headers.join("\t"));
    for row in &table.rows {
        lines.push(row.join("\t"));
    }
    lines.join("\n")
}

/// Convenience: markdown text -> TSV text.
pub fn markdown_to_tsv(md_text: &str) -> Result<String, TableError> {
    let md = parse_markdown_table(md_text)?;
    Ok(tsv_to_string(&md.table))
}

/// Convenience: TSV text -> markdown table text.
pub fn tsv_to_markdown(tsv_text: &str) -> Result<String, TableError> {
    let table = parse_tsv(tsv_text)?;
    if table.headers.is_empty() {
        return Ok(String::new());
    }
    let alignments = vec![Alignment::Default; table.headers.len()];
    Ok(markdown_table_to_string(&MarkdownTable {
        table,
        alignments,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_table() {
        let md = parse_markdown_table("| a | b |\n|---|---|\n| 1 | 2 |\n").unwrap();
        assert_eq!(md.table.headers, vec!["a", "b"]);
        assert_eq!(md.table.rows, vec![vec!["1", "2"]]);
        assert_eq!(md.alignments, vec![Alignment::Default, Alignment::Default]);
    }

    #[test]
    fn parses_table_without_outer_pipes() {
        let md = parse_markdown_table("a | b\n--- | ---\n1 | 2\n").unwrap();
        assert_eq!(md.table.headers, vec!["a", "b"]);
        assert_eq!(md.table.rows, vec![vec!["1", "2"]]);
    }

    #[test]
    fn parses_alignment() {
        let md =
            parse_markdown_table("| a | b | c | d |\n|:---|:---:|---:|---|\n|1|2|3|4|\n").unwrap();
        assert_eq!(
            md.alignments,
            vec![
                Alignment::Left,
                Alignment::Center,
                Alignment::Right,
                Alignment::Default
            ]
        );
    }

    #[test]
    fn handles_escaped_pipes() {
        let md = parse_markdown_table("| a | b |\n|---|---|\n| x \\| y | z |\n").unwrap();
        assert_eq!(md.table.rows[0][0], "x | y");
        let rendered = markdown_table_to_string(&md);
        assert!(rendered.contains("x \\| y"));
        // Round-trips.
        let md2 = parse_markdown_table(&rendered).unwrap();
        assert_eq!(md2.table, md.table);
    }

    #[test]
    fn handles_single_column_without_pipes() {
        let md = parse_markdown_table("h\n---\n1\n2\n").unwrap();
        assert_eq!(md.table.headers, vec!["h"]);
        assert_eq!(md.table.rows, vec![vec!["1"], vec!["2"]]);
    }

    #[test]
    fn rejects_missing_delimiter() {
        let err = parse_markdown_table("| a |\n").unwrap_err();
        assert!(matches!(err, TableError::ExpectedDelimiter { .. }));
    }

    #[test]
    fn rejects_inconsistent_columns() {
        let err = parse_markdown_table("| a | b |\n|---|---|\n| 1 |\n").unwrap_err();
        assert_eq!(
            err,
            TableError::InconsistentColumns {
                line: 3,
                expected: 2,
                actual: 1
            }
        );
    }

    #[test]
    fn tsv_round_trip() {
        let tsv = "a\tb\n1\t2\n";
        let table = parse_tsv(tsv).unwrap();
        assert_eq!(table.headers, vec!["a", "b"]);
        assert_eq!(table.rows, vec![vec!["1", "2"]]);
        assert_eq!(tsv_to_string(&table), "a\tb\n1\t2");
    }

    #[test]
    fn tsv_empty_input_gives_empty_table() {
        let table = parse_tsv("").unwrap();
        assert!(table.is_empty());
        assert_eq!(tsv_to_string(&table), "");
    }

    #[test]
    fn markdown_to_tsv_to_markdown_round_trip() {
        let md = "| a | b |\n|---|---|\n| 1 | 2 |\n";
        let tsv = markdown_to_tsv(md).unwrap();
        assert_eq!(tsv, "a\tb\n1\t2");
        let md2 = tsv_to_markdown(&tsv).unwrap();
        let parsed = parse_markdown_table(&md2).unwrap();
        assert_eq!(parsed.table, parse_markdown_table(md).unwrap().table);
    }
}
