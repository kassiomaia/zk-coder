//! Coder - AI Assistant Desktop Application
//!
//! This is the main library crate for the Coder application backend.

pub mod agents;
pub mod ai;
pub mod api;
pub mod commands;
pub mod keychain;
pub mod nlp;
pub mod storage;
pub mod utils;

use commands::{ChatState, ConfigState, NlpState};
use std::path::PathBuf;
use tauri::Manager;

/// Get the app data directory for storing databases.
fn get_app_data_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    // Use Tauri's app data directory
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    // Create the directory if it doesn't exist
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create data directory: {}", e))?;

    Ok(data_dir)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Get the app data directory
            let data_dir = get_app_data_dir(app.handle())?;

            // Initialize NLP state
            let nlp_state = NlpState::new_dev(&data_dir.join("coder_context.db").to_string_lossy())
                .expect("Failed to initialize NLP state");

            // Initialize Chat state
            let chat_state = ChatState::new(&data_dir.join("coder_chat.db").to_string_lossy())
                .expect("Failed to initialize Chat state");

            // Initialize Config state
            let config_state = ConfigState::new(&data_dir.join("coder_config.db").to_string_lossy())
                .expect("Failed to initialize Config state");

            // Manage states
            app.manage(nlp_state);
            app.manage(chat_state);
            app.manage(config_state);

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
            // Core commands
            commands::health_check,
            commands::get_app_info,
            // NLP commands
            commands::nlp::classify_intent,
            commands::nlp::get_conversation_context,
            commands::nlp::add_context_message,
            commands::nlp::detect_multi_intent,
            commands::nlp::detect_ambiguities,
            commands::nlp::analyze_code,
            commands::nlp::update_project_files,
            commands::nlp::clear_conversation_context,
            // Chat commands
            commands::chat::load_conversations,
            commands::chat::create_conversation,
            commands::chat::load_conversation,
            commands::chat::rename_conversation,
            commands::chat::delete_conversation,
            commands::chat::load_threads,
            commands::chat::create_thread,
            commands::chat::resolve_thread,
            commands::chat::delete_thread,
            commands::chat::load_messages,
            commands::chat::load_thread_messages,
            commands::chat::send_message,
            commands::chat::delete_message,
            commands::chat::update_message_status,
            commands::chat::create_thread_with_message,
            // Config commands
            commands::config::get_providers,
            commands::config::save_provider,
            commands::config::remove_provider,
            commands::config::validate_api_key,
            commands::config::test_connection,
            commands::config::fetch_models,
            commands::config::store_api_key,
            commands::config::retrieve_api_key,
            commands::config::delete_api_key,
            commands::config::has_api_key,
            commands::config::get_operation_defaults,
            commands::config::set_operation_default,
            commands::config::get_models,
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
