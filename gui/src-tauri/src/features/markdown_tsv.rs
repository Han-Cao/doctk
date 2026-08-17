//! Tauri commands for the markdown <-> TSV tool.

use doctk_core::features::markdown_tsv::{self, Alignment, MarkdownTable, Table};

#[tauri::command]
pub fn markdown_tsv_md_to_table(md: String) -> Result<Table, String> {
    markdown_tsv::parse_markdown_table(&md)
        .map(|parsed| parsed.table)
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn markdown_tsv_table_to_md(table: Table) -> Result<String, String> {
    let alignments = vec![Alignment::Default; table.headers.len()];
    Ok(markdown_tsv::markdown_table_to_string(&MarkdownTable {
        table,
        alignments,
    }))
}
