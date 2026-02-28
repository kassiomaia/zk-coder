//! Coder - AI Assistant Desktop Application
//!
//! This is the main library crate for the Coder application backend.

pub mod agents;
pub mod commands;
pub mod utils;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::health_check,
            commands::get_app_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check() {
        let result = commands::health_check();
        assert!(result.contains("Coder backend is running"));
    }

    #[test]
    fn test_get_app_info() {
        let info = commands::get_app_info();
        assert_eq!(info.name, "coder");
        assert!(!info.version.is_empty());
    }
}
