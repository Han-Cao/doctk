//! Tauri command for the case converter tool.

use doctk_core::features::case_converter::{self, CaseMode};

#[tauri::command]
pub fn case_converter_convert(
    text: String,
    mode: CaseMode,
    proper_nouns: Vec<String>,
) -> Result<String, String> {
    Ok(case_converter::convert(&text, mode, &proper_nouns))
}
