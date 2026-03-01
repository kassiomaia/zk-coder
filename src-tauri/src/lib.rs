//! Coder - AI Assistant Desktop Application
//!
//! This is the main library crate for the Coder application backend.

pub mod agents;
pub mod ai;
pub mod commands;
pub mod nlp;
pub mod storage;
pub mod utils;

use commands::{ChatState, NlpState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize NLP state
    let nlp_state = NlpState::new_dev("coder_context.db")
        .expect("Failed to initialize NLP state");

    // Initialize Chat state
    let chat_state = ChatState::new("coder_chat.db")
        .expect("Failed to initialize Chat state");

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
        .manage(nlp_state)
        .manage(chat_state)
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
