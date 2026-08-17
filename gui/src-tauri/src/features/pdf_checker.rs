//! Tauri commands for the PDF checker tool.

use doctk_core::features::pdf_checker::{self, PdfFileReport};
use doctk_core::units::PaperSize;
use std::path::PathBuf;

#[tauri::command]
pub async fn pdf_checker_check_files(
    paths: Vec<String>,
    paper: PaperSize,
    tolerance_pt: f64,
    ignore_orientation: bool,
) -> Result<Vec<PdfFileReport>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
        Ok(pdf_checker::inspect_pdfs(
            &paths,
            &paper,
            tolerance_pt,
            ignore_orientation,
        ))
    })
    .await
    .map_err(|err| err.to_string())?
}
