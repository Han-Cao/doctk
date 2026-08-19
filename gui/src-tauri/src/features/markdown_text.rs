//! Tauri command for the markdown -> text tool.

use doctk_core::features::markdown_text;

#[tauri::command]
pub fn markdown_text_convert(md: String) -> Result<String, String> {
    Ok(markdown_text::markdown_to_text(&md))
}
