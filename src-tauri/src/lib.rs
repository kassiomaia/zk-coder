//! Coder - AI Assistant Desktop Application
//!
//! This is the main library crate for the Coder application backend.

// Core domain layer - foundational entities and interfaces
pub mod core;

// Application layer - service implementations
pub mod application;

pub mod commands;
pub mod infrastructure;
#[cfg(test)]
mod test_support;

use commands::{AgentState, ChatState, ConfigState, LlmState, NlpState};
use infrastructure::sqlite::connection;
use infrastructure::sqlite::migrations;
use infrastructure::sqlite::repositories::config::SqliteConfigRepository;
use infrastructure::tools::documents::shared::ArtifactStore;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

use application::llm::LlmServiceImpl;

/// Retrieves the application's data directory, creating it if it doesn't exist.
///
/// This function is used to get a platform-specific and user-specific directory
/// for storing application data, such as databases and configuration files.
///
/// # Arguments
///
/// * `app_handle` - A handle to the Tauri application instance.
///
/// # Returns
///
/// A `Result` containing the `PathBuf` of the data directory on success,
/// or a `String` with an error message on failure.
fn get_app_data_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    // Use Tauri's app data directory
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {e}"))?;

    // Create the directory if it doesn't exist
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create data directory: {e}"))?;

    Ok(data_dir)
}

/// The main entry point for the Coder Tauri application.
///
/// This function sets up and runs the Tauri application. It initializes the
/// application builder, sets up the database connections, initializes all
/// the application states and services, configures plugins, and registers
/// all the Tauri commands.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Get the app data directory
            let data_dir = get_app_data_dir(app.handle())?;
            let db_path = data_dir.join("coder.db");

            // Open a single shared connection for all domains
            let conn = connection::open_shared(&db_path.to_string_lossy())
                .expect("Failed to open database");

            // Run all schema initialisation and migrations in one pass
            {
                let db = conn.lock().expect("Failed to acquire database lock");
                migrations::init_all(&db).expect("Failed to initialize database schema");
            }

            // Build the shared config repository (seeding happens inside from_shared)
            let config_repository = Arc::new(
                SqliteConfigRepository::from_shared(conn.clone())
                    .expect("Failed to initialize config repository"),
            );

            // Best-effort: purge expired web_search cache entries on startup
            // so the DB stays tidy. A failure here must not block launch.
            match config_repository.sweep_expired_web_search_cache(chrono::Utc::now()) {
                Ok(removed) if removed > 0 => {
                    log::info!(
                        target: "web_search",
                        "startup sweep removed {removed} expired cache entries"
                    );
                }
                Ok(_) => {}
                Err(err) => {
                    log::warn!(
                        target: "web_search",
                        "startup sweep of expired cache entries failed: {err}"
                    );
                }
            }

            // Initialize NLP state
            let nlp_state = NlpState::from_shared(conn.clone(), config_repository.clone())
                .expect("Failed to initialize NLP state");

            // Initialize Chat state
            let chat_state = ChatState::from_shared(conn.clone())
                .expect("Failed to initialize Chat state");

            // Initialize Config state
            let config_state = ConfigState::from_repository(
                config_repository.clone() as Arc<dyn crate::core::config::ConfigRepository>,
            );

            let llm_service = Arc::new(LlmServiceImpl::new(config_repository.clone()))
                as Arc<dyn crate::core::llm::services::LlmService>;

            // Initialize LLM state
            let llm_state = LlmState::from_repository(
                config_repository.clone() as Arc<dyn crate::core::config::ConfigRepository>,
                llm_service,
            );

            // Initialize Agent state
            let agent_state = AgentState::from_shared(
                conn.clone(),
                chat_state.service.clone(),
                config_repository.clone() as Arc<dyn crate::core::config::ConfigRepository>,
            )
            .expect("Failed to initialize Agent state");

            // Manage states
            app.manage(nlp_state);
            app.manage(chat_state);
            app.manage(config_state);
            app.manage(llm_state);
            app.manage(agent_state);

            // Initialize and manage artifact store
            let artifact_store = Arc::new(std::sync::Mutex::new(ArtifactStore::new()));
            app.manage(artifact_store);

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

            // Register opener plugin (used by web search results to open URLs in the
            // user's default browser). Scope-limited via `capabilities/default.json`.
            app.handle().plugin(tauri_plugin_opener::init())?;

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
            commands::chat::conversations::archive_conversation,
            commands::chat::threads::load_threads,
            commands::chat::threads::create_thread,
            commands::chat::threads::resolve_thread,
            commands::chat::threads::archive_thread,
            commands::chat::threads::delete_thread,
            commands::chat::threads::rename_thread,
            commands::chat::threads::set_thread_agent_session,
            commands::chat::threads::set_yolo_mode,
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
            // Web search tool config commands
            commands::config::tool_credentials::set_tool_credentials,
            commands::config::tool_credentials::get_tool_credentials,
            commands::config::tool_credentials::delete_tool_credentials,
            commands::config::tool_credentials::has_tool_credentials,
            commands::config::tool_credentials::test_web_search_credentials,
            commands::config::workspace_policy::get_workspace_search_policy,
            commands::config::workspace_policy::set_workspace_search_policy,
            commands::config::workspace_policy::delete_workspace_search_policy,
            commands::config::web_search_cache::clear_web_search_cache,
            commands::config::web_search_cache::web_search_cache_stats,
            commands::config::pdf_theme::list_pdf_themes,
            commands::config::pdf_theme::get_pdf_theme,
            commands::config::pdf_theme::set_pdf_theme,
            // LLM commands
            commands::llm::commands::send_chat_message,
            commands::llm::commands::send_chat_message_streaming,
            commands::llm::commands::cancel_chat_request,
            // Agent commands
            commands::agent::sessions::create_agent_session,
            commands::agent::sessions::send_agent_request,
            commands::agent::sessions::reply_to_clarification,
            commands::agent::sessions::retry_last_agent_request,
            commands::agent::sessions::continue_agent_execution,
            commands::agent::sessions::replan_agent_execution,
            commands::agent::sessions::initialize_agent_recovery,
            commands::agent::sessions::get_agent_session,
            commands::agent::sessions::get_agent_sessions_for_conversation,
            commands::agent::sessions::cancel_agent_session,
            commands::agent::sessions::delete_agent_session,
            commands::agent::sessions::update_agent_session_workspace,
            commands::agent::sessions::update_agent_session_model,
            commands::agent::approvals::approve_agent_action,
            commands::agent::approvals::deny_agent_action,
            commands::agent::approvals::get_pending_agent_actions,
            commands::agent::registry::list_agents,
            commands::agent::registry::get_agent,
            commands::agent::debug::open_agent_trace_window,
            commands::agent::debug::get_agent_execution_debug,
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
            commands::file::open_file_diff_window,
            commands::workspace_files::list_workspace_dir,
            // Session history commands
            commands::agent::history::get_conversation_session_history,
            commands::agent::history::get_session_details,
            // Document artifact commands
            commands::documents::list_artifacts,
            commands::documents::list_all_artifacts,
            commands::documents::download_artifact,
            commands::documents::save_artifact_to_path,
            commands::documents::get_artifact,
            commands::documents::save_artifact,
            commands::documents::update_metadata,
            commands::documents::delete_artifact,
            commands::documents::cleanup_session,
            commands::documents::count_session,
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
