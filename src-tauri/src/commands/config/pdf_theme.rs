//! Tauri commands for PDF theme selection (T3.3).

use crate::commands::config::state::ConfigState;
use crate::core::config::PdfThemeRepository;

/// Lists the names of all bundled PDF themes.
#[tauri::command]
pub async fn list_pdf_themes() -> Result<Vec<String>, String> {
    Ok(vec!["report".to_string()])
}

/// Returns the theme name selected for `workspace_path`, or `None` if
/// the user has not overridden the default.
#[tauri::command]
pub async fn get_pdf_theme(
    workspace_path: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<Option<String>, String> {
    state
        .repository
        .get_pdf_theme(&workspace_path)
        .map_err(|e| format!("db error: {e}"))
}

/// Persists the selected theme for `workspace_path`.
#[tauri::command]
pub async fn set_pdf_theme(
    workspace_path: String,
    theme_name: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    state
        .repository
        .set_pdf_theme(&workspace_path, &theme_name)
        .map_err(|e| format!("db error: {e}"))
}
