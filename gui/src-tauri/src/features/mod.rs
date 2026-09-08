//! Tauri command registry.  Each feature module exposes `#[tauri::command]`
//! functions; registering a new tool means adding its commands here.

pub mod case_converter;
pub mod diff_checker;
pub mod markdown_text;
pub mod markdown_tsv;
pub mod pdf_checker;

pub fn handlers() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        markdown_tsv::markdown_tsv_md_to_table,
        markdown_tsv::markdown_tsv_parse_tsv,
        markdown_tsv::markdown_tsv_table_to_md,
        markdown_text::markdown_text_convert,
        diff_checker::diff_checker_side_by_side,
        diff_checker::diff_checker_track_changes,
        case_converter::case_converter_convert,
        pdf_checker::pdf_checker_check_files,
    ]
}
