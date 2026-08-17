//! Tauri command registry.  Each feature module exposes `#[tauri::command]`
//! functions; registering a new tool means adding its commands here.

pub mod diff_checker;
pub mod markdown_tsv;
pub mod pdf_checker;

pub fn handlers() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        markdown_tsv::markdown_tsv_md_to_table,
        markdown_tsv::markdown_tsv_table_to_md,
        diff_checker::diff_checker_side_by_side,
        diff_checker::diff_checker_track_changes,
        pdf_checker::pdf_checker_check_files,
    ]
}
