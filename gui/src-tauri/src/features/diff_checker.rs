//! Tauri commands for the diff checker tool.

use doctk_core::features::diff_checker::{self, SideBySideDiff, TrackChangesDiff};

#[tauri::command]
pub fn diff_checker_side_by_side(left: String, right: String) -> Result<SideBySideDiff, String> {
    diff_checker::validate_input_size(&left, &right).map_err(|err| err.to_string())?;
    Ok(diff_checker::diff_side_by_side(&left, &right))
}

#[tauri::command]
pub fn diff_checker_track_changes(left: String, right: String) -> Result<TrackChangesDiff, String> {
    diff_checker::validate_input_size(&left, &right).map_err(|err| err.to_string())?;
    Ok(diff_checker::diff_track_changes(&left, &right))
}
