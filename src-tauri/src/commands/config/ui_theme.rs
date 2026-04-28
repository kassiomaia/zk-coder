//! Tauri commands for the global UI theme preference.
//!
//! The frontend persists the user's choice via `set_ui_theme` and reads it
//! back on boot via `get_ui_theme`. The value is one of `light`, `dark`, or
//! `system`; the layer above this module is the source of truth for the
//! allowed values, but the database also enforces the constraint.

use crate::commands::config::state::ConfigState;

const ALLOWED_THEMES: &[&str] = &["light", "dark", "system"];

/// Returns the persisted UI theme, or `None` when the user has not set one yet.
#[tauri::command]
pub async fn get_ui_theme(state: tauri::State<'_, ConfigState>) -> Result<Option<String>, String> {
    state
        .repository
        .get_ui_theme()
        .map_err(|e| format!("db error: {e}"))
}

/// Persists the UI theme. Rejects unknown values up-front so the backend never
/// has to round-trip an invalid theme string.
#[tauri::command]
pub async fn set_ui_theme(
    theme: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    if !ALLOWED_THEMES.contains(&theme.as_str()) {
        return Err(format!(
            "invalid theme '{theme}'; expected one of: {}",
            ALLOWED_THEMES.join(", ")
        ));
    }
    state
        .repository
        .set_ui_theme(&theme)
        .map_err(|e| format!("db error: {e}"))
}
