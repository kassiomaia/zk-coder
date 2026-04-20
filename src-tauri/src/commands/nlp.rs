//! NLP Tauri IPC commands.
//!
//! This module provides Tauri commands for NLP functionality.

use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use tokio::sync::Mutex;

use rusqlite::Connection;

use crate::application::nlp::{
    AmbiguityResolver, CodeAnalyzer, ContextManager, IntentClassifier, MultiIntentDetector,
    NlpLlmGateway,
};
use crate::core::config::ConfigRepository;
use crate::core::chat::entities::MessageRole;
use crate::core::nlp::ambiguity::{Ambiguity, ResolutionType, ResolvedAmbiguity};
use crate::core::nlp::code_understanding::{CodeUnderstanding, ProgrammingLanguage};
use crate::core::nlp::entities::{ContextMessage, ConversationContext};
use crate::core::nlp::intent::IntentClassification;
use crate::core::nlp::multi_intent::MultiIntentResult;
use crate::core::nlp::repositories::ContextWindowRepository;
use crate::infrastructure::nlp::llm_gateway::ConfigBackedNlpLlmGateway;
use crate::infrastructure::sqlite::repositories::nlp_context::SqliteContextWindowRepository;

/// Application state for NLP commands.
pub struct NlpState {
    /// Intent classifier
    pub classifier: IntentClassifier,
    /// Context manager
    pub context_manager: ContextManager,
    /// Multi-intent detector
    pub multi_detector: MultiIntentDetector,
    /// Ambiguity resolver
    pub ambiguity_resolver: AmbiguityResolver,
    /// Code analyzer
    pub code_analyzer: CodeAnalyzer,
    /// Project files for ambiguity resolution
    pub project_files: Arc<Mutex<Vec<String>>>,
}

impl NlpState {
    /// Creates a new NLP state.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    pub fn new(config_db_path: &str, context_db_path: &str) -> Result<Self, String> {
        let gateway: Arc<dyn NlpLlmGateway + Send + Sync> =
            Arc::new(ConfigBackedNlpLlmGateway::new(config_db_path).map_err(|e| e.to_string())?);
        let store = Arc::new(
            SqliteContextWindowRepository::new(context_db_path).map_err(|e| e.to_string())?,
        ) as Arc<dyn ContextWindowRepository>;
        let code_analyzer = CodeAnalyzer::new().map_err(|e| e.to_string())?;

        Ok(Self {
            classifier: IntentClassifier::new(Arc::clone(&gateway)),
            context_manager: ContextManager::new(store, Arc::clone(&gateway)),
            multi_detector: MultiIntentDetector::new(gateway),
            ambiguity_resolver: AmbiguityResolver::new(),
            code_analyzer,
            project_files: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Creates an NLP state from a shared connection and existing config repository.
    ///
    /// The schema must already have been applied via `migrations::init_all`.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    pub fn from_shared(
        conn: Arc<StdMutex<Connection>>,
        config_repository: Arc<dyn ConfigRepository>,
    ) -> Result<Self, String> {
        let gateway: Arc<dyn NlpLlmGateway + Send + Sync> = Arc::new(
            ConfigBackedNlpLlmGateway::from_repository(config_repository),
        );
        let store = Arc::new(SqliteContextWindowRepository::from_shared(conn))
            as Arc<dyn ContextWindowRepository>;
        let code_analyzer = CodeAnalyzer::new().map_err(|e| e.to_string())?;

        Ok(Self {
            classifier: IntentClassifier::new(Arc::clone(&gateway)),
            context_manager: ContextManager::new(store, Arc::clone(&gateway)),
            multi_detector: MultiIntentDetector::new(gateway),
            ambiguity_resolver: AmbiguityResolver::new(),
            code_analyzer,
            project_files: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Creates a new NLP state with in-memory storage (tests only).
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    pub fn in_memory() -> Result<Self, String> {
        let gateway: Arc<dyn NlpLlmGateway + Send + Sync> =
            Arc::new(ConfigBackedNlpLlmGateway::in_memory().map_err(|e| e.to_string())?);
        let store = Arc::new(SqliteContextWindowRepository::in_memory().map_err(|e| e.to_string())?)
            as Arc<dyn ContextWindowRepository>;
        let code_analyzer = CodeAnalyzer::new().map_err(|e| e.to_string())?;

        Ok(Self {
            classifier: IntentClassifier::new(Arc::clone(&gateway)),
            context_manager: ContextManager::new(store, Arc::clone(&gateway)),
            multi_detector: MultiIntentDetector::new(gateway),
            ambiguity_resolver: AmbiguityResolver::new(),
            code_analyzer,
            project_files: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Creates a new NLP state using a single development DB path.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    pub fn new_dev(db_path: &str) -> Result<Self, String> {
        Self::new(db_path, db_path)
    }
}

/// Classifies intent from user input.
#[tauri::command]
pub async fn classify_intent(
    input: String,
    conversation_id: String,
    state: tauri::State<'_, NlpState>,
) -> Result<IntentClassification, String> {
    let context = state
        .context_manager
        .get_context(&conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    state
        .classifier
        .classify(&input, &context)
        .await
        .map_err(|e| e.to_string())
}

/// Gets context for a conversation.
#[tauri::command]
pub async fn get_conversation_context(
    conversation_id: String,
    state: tauri::State<'_, NlpState>,
) -> Result<ConversationContext, String> {
    state
        .context_manager
        .get_context(&conversation_id)
        .await
        .map_err(|e| e.to_string())
}

/// Adds a message to conversation context.
#[tauri::command]
pub async fn add_context_message(
    conversation_id: String,
    role: String,
    content: String,
    state: tauri::State<'_, NlpState>,
) -> Result<(), String> {
    let message = ContextMessage {
        id: uuid::Uuid::new_v4().to_string(),
        role: match role.as_str() {
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            _ => MessageRole::System,
        },
        content,
        timestamp: chrono::Utc::now(),
        tokens: None,
    };

    state
        .context_manager
        .add_message(&conversation_id, message)
        .await
        .map_err(|e| e.to_string())
}

/// Detects multiple intents in input.
#[tauri::command]
pub async fn detect_multi_intent(
    input: String,
    conversation_id: String,
    state: tauri::State<'_, NlpState>,
) -> Result<MultiIntentResult, String> {
    let context = state
        .context_manager
        .get_context(&conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    state
        .multi_detector
        .detect(&input, &context)
        .await
        .map_err(|e| e.to_string())
}

/// Detects ambiguities in input.
#[tauri::command]
pub async fn detect_ambiguities(
    input: String,
    conversation_id: String,
    state: tauri::State<'_, NlpState>,
) -> Result<Vec<Ambiguity>, String> {
    let context = state
        .context_manager
        .get_context(&conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    let files = state.project_files.lock().await.clone();

    state
        .ambiguity_resolver
        .detect(&input, &context, &files)
        .await
        .map_err(|e| e.to_string())
}

/// Analyzes code for understanding.
#[tauri::command]
pub fn analyze_code(
    code: String,
    language: ProgrammingLanguage,
    state: tauri::State<'_, NlpState>,
) -> Result<CodeUnderstanding, String> {
    state
        .code_analyzer
        .analyze(&code, language)
        .map_err(|e| e.to_string())
}

/// Updates project files for ambiguity resolution.
#[tauri::command]
pub async fn update_project_files(
    files: Vec<String>,
    state: tauri::State<'_, NlpState>,
) -> Result<(), String> {
    let mut project_files = state.project_files.lock().await;
    *project_files = files;
    Ok(())
}

/// Clears a conversation context.
#[tauri::command]
pub fn clear_conversation_context(
    conversation_id: String,
    state: tauri::State<'_, NlpState>,
) -> Result<(), String> {
    state
        .context_manager
        .clear(&conversation_id)
        .map_err(|e| e.to_string())
}

/// Resolves an ambiguity with the provided resolution.
///
/// This creates a `ResolvedAmbiguity` record for tracking purposes.
#[tauri::command]
pub async fn resolve_ambiguity(
    ambiguity_id: String,
    original_text: String,
    resolution: String,
    resolution_type: String,
) -> Result<ResolvedAmbiguity, String> {
    let res_type = match resolution_type.as_str() {
        "selected" => ResolutionType::Selected,
        "custom" => ResolutionType::Custom,
        "dismissed" => ResolutionType::Dismissed,
        _ => return Err(format!("Invalid resolution type: {resolution_type}")),
    };

    Ok(ResolvedAmbiguity::new(
        ambiguity_id,
        original_text,
        resolution,
        res_type,
    ))
}

/// Applies multiple resolutions to input text.
///
/// Takes the original input and a map of ambiguity IDs to their resolutions,
/// returns the modified input with all resolutions applied.
#[tauri::command]
pub async fn apply_resolutions(
    input: String,
    resolutions: std::collections::HashMap<String, String>,
) -> Result<String, String> {
    let mut result = input;

    // Apply resolutions by replacing the original text with the resolution
    // Note: This is a simple replacement strategy. More sophisticated approaches
    // could use span information for precise replacement.
    for (_ambiguity_id, resolution) in resolutions {
        // If the resolution is empty (dismissed), skip it
        if resolution.is_empty() {
            continue;
        }
        // For now, we just return the resolution as-is since we don't have
        // the original span info in this simplified approach.
        // A more sophisticated implementation would track spans and do precise replacement.
        result = resolution;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nlp_state_creation() {
        let state = NlpState::in_memory();
        assert!(state.is_ok());
    }
}
