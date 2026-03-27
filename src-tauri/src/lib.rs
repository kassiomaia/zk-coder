//! Coder - AI Assistant Desktop Application
//!
//! This is the main library crate for the Coder application backend.

// Core domain layer - foundational entities and interfaces
pub mod core;

// Application layer - service implementations
pub mod application;

pub mod commands;
pub mod infrastructure;

use commands::{AgentState, ChatState, ConfigState, LlmState, NlpState};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

use application::llm::LlmServiceImpl;
use infrastructure::sqlite::repositories::config::SqliteConfigRepository;

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
            let config_db = data_dir.join("coder_config.db");
            let context_db = data_dir.join("coder_context.db");
            let chat_db = data_dir.join("coder_chat.db");

            // Initialize NLP state
            let nlp_state =
                NlpState::new(&config_db.to_string_lossy(), &context_db.to_string_lossy())
                    .expect("Failed to initialize NLP state");

            // Initialize Chat state
            let chat_state = ChatState::new(&chat_db.to_string_lossy())
                .expect("Failed to initialize Chat state");

            // Initialize Config state
            let config_state = ConfigState::new(&config_db.to_string_lossy())
                .expect("Failed to initialize Config state");

            // Initialize LLM service (uses same database as config)
            let config_repository = Arc::new(
                SqliteConfigRepository::new(&config_db.to_string_lossy())
                    .expect("Failed to initialize SqliteConfigRepository"),
            );
            let llm_service = Arc::new(LlmServiceImpl::new(config_repository.clone()))
                as Arc<dyn crate::core::llm::services::LlmService>;

            // Initialize LLM state
            let llm_state = LlmState::new(&config_db.to_string_lossy(), llm_service)
                .expect("Failed to initialize LLM state");

            // Initialize Agent state
            let agent_db = data_dir.join("coder_agents.db");
            let agent_state = AgentState::new(&agent_db.to_string_lossy())
                .expect("Failed to initialize Agent state");

            // Manage states
            app.manage(nlp_state);
            app.manage(chat_state);
            app.manage(config_state);
            app.manage(llm_state);
            app.manage(agent_state);

            // Configure logging - always log to file, also log to stdout in debug mode
            let log_targets = if cfg!(debug_assertions) {
                vec![
                    Target::new(TargetKind::LogDir { file_name: None }),
                    Target::new(TargetKind::Stdout),
                ]
            } else {
                vec![Target::new(TargetKind::LogDir { file_name: None })]
            };

            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .targets(log_targets)
                    .build(),
            )?;

            // Register notification plugin
            app.handle().plugin(tauri_plugin_notification::init())?;

            // Register dialog plugin (for workspace folder selection)
            app.handle().plugin(tauri_plugin_dialog::init())?;

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
            commands::nlp::resolve_ambiguity,
            commands::nlp::apply_resolutions,
            // Chat commands
            commands::chat::conversations::load_conversations,
            commands::chat::conversations::create_conversation,
            commands::chat::conversations::load_conversation,
            commands::chat::conversations::rename_conversation,
            commands::chat::conversations::delete_conversation,
            commands::chat::conversations::update_conversation_settings,
            commands::chat::conversations::set_conversation_workspace,
            commands::chat::threads::load_threads,
            commands::chat::threads::create_thread,
            commands::chat::threads::resolve_thread,
            commands::chat::threads::delete_thread,
            commands::chat::threads::rename_thread,
            commands::chat::threads::set_thread_agent_session,
            commands::chat::messages::load_messages,
            commands::chat::messages::load_thread_messages,
            commands::chat::messages::send_message,
            commands::chat::messages::delete_message,
            commands::chat::messages::update_message_status,
            commands::chat::messages::update_message_feedback,
            commands::chat::threads::create_thread_with_message,
            // Config commands
            commands::config::providers::get_providers,
            commands::config::providers::save_provider,
            commands::config::providers::remove_provider,
            commands::config::api_keys::validate_api_key,
            commands::config::api_keys::test_connection,
            commands::config::api_keys::fetch_models,
            commands::config::api_keys::store_api_key,
            commands::config::api_keys::retrieve_api_key,
            commands::config::api_keys::delete_api_key,
            commands::config::api_keys::has_api_key,
            commands::config::defaults::get_operation_defaults,
            commands::config::defaults::set_operation_default,
            commands::config::models::get_models,
            commands::config::models::get_manual_models,
            commands::config::models::add_manual_model,
            commands::config::models::remove_manual_model,
            commands::config::models::set_provider_models_source,
            // LLM commands
            commands::llm::commands::send_chat_message,
            commands::llm::commands::send_chat_message_streaming,
            commands::llm::commands::cancel_chat_request,
            // Agent commands
            commands::agent::sessions::create_agent_session,
            commands::agent::sessions::send_agent_request,
            commands::agent::sessions::retry_last_agent_request,
            commands::agent::sessions::get_agent_session,
            commands::agent::sessions::get_agent_sessions_for_conversation,
            commands::agent::sessions::cancel_agent_session,
            commands::agent::sessions::delete_agent_session,
            commands::agent::sessions::update_agent_session_workspace,
            commands::agent::approvals::approve_agent_action,
            commands::agent::approvals::deny_agent_action,
            commands::agent::approvals::get_pending_agent_actions,
            commands::agent::registry::list_agents,
            commands::agent::registry::get_agent,
            // Agent rollback commands
            commands::agent::rollback::get_session_file_snapshots,
            commands::agent::rollback::rollback_file,
            commands::agent::rollback::rollback_session,
            // Agent subtask commands
            commands::agent::subtasks::create_subtask,
            commands::agent::subtasks::update_subtask_status,
            commands::agent::subtasks::get_subtasks,
            // File viewing commands (UI-only, no LLM consumption)
            commands::file::get_file_content,
            // Session history commands
            commands::agent::history::get_conversation_session_history,
            commands::agent::history::get_session_details,
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
