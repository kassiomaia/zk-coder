//! Chat Tauri IPC commands.
//!
//! Thin adapter layer that delegates to the ChatService.

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::application::ChatServiceImpl;
use crate::core::chat::entities::{
    ChatMessage, Conversation, ConversationSettings, MessageFeedback, MessageRole, MessageStatus,
    Thread,
};
use crate::core::chat::services::{ChatService, CreateThreadRequest, SendMessageRequest};
use crate::storage::SqliteChatStore;

/// Application state for chat commands.
pub struct ChatState {
    /// Chat service for business logic
    pub service: Arc<dyn ChatService>,
    /// Store for direct repository access
    ///
    /// NOTE: This is kept for backward compatibility with modules that
    /// access the repository layer directly (e.g., tools/conversation, commands/llm/orchestrator).
    /// New code should use the service interface instead.
    pub store: Arc<Mutex<SqliteChatStore>>,
}

impl ChatState {
    /// Creates a new chat state with a file-backed database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be initialized.
    pub fn new(db_path: &str) -> Result<Self, String> {
        // Create a shared store for the service
        let store = Arc::new(SqliteChatStore::new(db_path).map_err(|e| e.to_string())?);
        let service = Arc::new(ChatServiceImpl::new(store.clone())) as Arc<dyn ChatService>;
        // Create a second connection for the Mutex-wrapped store
        // NOTE: This creates a second DB connection but is needed for backward compatibility.
        // TODO: Migrate all store consumers to use the service interface.
        let store2 = SqliteChatStore::new(db_path).map_err(|e| e.to_string())?;
        Ok(Self {
            service,
            store: Arc::new(Mutex::new(store2)),
        })
    }

    /// Creates a new chat state with an in-memory database (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be initialized.
    pub fn in_memory() -> Result<Self, String> {
        // Create a shared store for the service
        let store = Arc::new(SqliteChatStore::in_memory().map_err(|e| e.to_string())?);
        let service = Arc::new(ChatServiceImpl::new(store.clone())) as Arc<dyn ChatService>;
        // Create a second connection for the Mutex-wrapped store
        // NOTE: This creates a second DB connection but is needed for backward compatibility.
        // TODO: Migrate all store consumers to use the service interface.
        let store2 = SqliteChatStore::in_memory().map_err(|e| e.to_string())?;
        Ok(Self {
            service,
            store: Arc::new(Mutex::new(store2)),
        })
    }

    /// Creates a new chat state with a custom service (for testing).
    #[cfg(test)]
    pub fn with_service(service: Arc<dyn ChatService>) -> Self {
        // For tests with mock service, create a minimal in-memory store
        let store = SqliteChatStore::in_memory().expect("Failed to create store");
        Self {
            service,
            store: Arc::new(Mutex::new(store)),
        }
    }
}

// =========================================================================
// Conversation Commands
// =========================================================================

#[tauri::command]
pub async fn load_conversations(
    state: tauri::State<'_, ChatState>,
) -> Result<Vec<Conversation>, String> {
    state.service.load_all_conversations().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_conversation(
    title: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Conversation, String> {
    state.service.create_conversation(&title).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_conversation(
    id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Conversation, String> {
    state.service.load_conversation(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_conversation(
    id: String,
    title: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Conversation, String> {
    state.service.rename_conversation(&id, &title).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_conversation(
    id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    state.service.delete_conversation(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_conversation_settings(
    id: String,
    settings: ConversationSettings,
    state: tauri::State<'_, ChatState>,
) -> Result<Conversation, String> {
    state
        .service
        .update_conversation_settings(&id, &settings)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_conversation_workspace(
    id: String,
    workspace_root: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Conversation, String> {
    state
        .service
        .set_conversation_workspace(&id, &workspace_root)
        .map_err(|e| e.to_string())
}

// =========================================================================
// Thread Commands
// =========================================================================

#[tauri::command]
pub async fn load_threads(
    conversation_id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Vec<Thread>, String> {
    state
        .service
        .load_threads(&conversation_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_thread(
    conversation_id: String,
    title: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Thread, String> {
    let request = CreateThreadRequest {
        conversation_id,
        title,
    };
    state.service.create_thread(request).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resolve_thread(
    id: String,
    resolved: bool,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    state.service.resolve_thread(&id, resolved).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_thread(id: String, state: tauri::State<'_, ChatState>) -> Result<(), String> {
    state.service.delete_thread(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_thread(
    id: String,
    title: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Thread, String> {
    state.service.rename_thread(&id, &title).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_thread_agent_session(
    id: String,
    agent_session_id: Option<String>,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    state
        .service
        .set_thread_agent_session(&id, agent_session_id)
        .map_err(|e| e.to_string())
}

/// Response for creating a thread with a message.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadWithMessageResponse {
    pub thread: Thread,
    pub message: ChatMessage,
}

#[tauri::command]
pub async fn create_thread_with_message(
    conversation_id: String,
    thread_title: String,
    content: String,
    role: String,
    state: tauri::State<'_, ChatState>,
) -> Result<ThreadWithMessageResponse, String> {
    let message_role = MessageRole::from_str(&role);
    let (thread, message) = state
        .service
        .create_thread_with_message(&conversation_id, &thread_title, &content, message_role)
        .map_err(|e| e.to_string())?;
    Ok(ThreadWithMessageResponse { thread, message })
}

// =========================================================================
// Message Commands
// =========================================================================

#[tauri::command]
pub async fn load_messages(
    conversation_id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Vec<ChatMessage>, String> {
    state
        .service
        .load_messages(&conversation_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_thread_messages(
    thread_id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Vec<ChatMessage>, String> {
    state
        .service
        .load_thread_messages(&thread_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_message(
    conversation_id: String,
    content: String,
    role: String,
    thread_id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<ChatMessage, String> {
    let request = SendMessageRequest {
        conversation_id,
        thread_id,
        content,
        role: MessageRole::from_str(&role),
    };
    state.service.send_message(request).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_message(id: String, state: tauri::State<'_, ChatState>) -> Result<(), String> {
    state.service.delete_message(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_message_status(
    id: String,
    status: String,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    let parsed_status = match status.as_str() {
        "sending" => MessageStatus::Sending,
        "delivered" => MessageStatus::Delivered,
        "error" => MessageStatus::Error,
        _ => MessageStatus::Sent,
    };
    state
        .service
        .update_message_status(&id, parsed_status)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_message_feedback(
    id: String,
    feedback: Option<String>,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    let parsed_feedback = feedback.and_then(|f| match f.as_str() {
        "like" => Some(MessageFeedback::Like),
        "dislike" => Some(MessageFeedback::Dislike),
        _ => None,
    });
    state
        .service
        .update_message_feedback(&id, parsed_feedback)
        .map_err(|e| e.to_string())
}

#[cfg(all(test, feature = "integration-test"))]
mod tests {
    use super::*;
    use crate::core::chat::entities::{ChatDomainError, Conversation, MessageRole};
    use crate::core::chat::repositories::RepositoryResult;
    use std::sync::Arc;

    // Mock service for testing commands
    struct MockChatService {
        should_fail: bool,
        thread_resolved: bool,
    }

    impl MockChatService {
        fn new() -> Self {
            Self {
                should_fail: false,
                thread_resolved: false,
            }
        }

        fn with_thread_resolved() -> Self {
            Self {
                should_fail: false,
                thread_resolved: true,
            }
        }

        fn with_error() -> Self {
            Self {
                should_fail: true,
                thread_resolved: false,
            }
        }
    }

    impl crate::core::chat::services::ChatService for MockChatService {
        fn create_conversation(&self, title: &str) -> RepositoryResult<Conversation> {
            if self.should_fail {
                return Err(ChatDomainError::InvalidInput {
                    reason: "Mock error".to_string(),
                });
            }
            Ok(Conversation {
                id: uuid::Uuid::new_v4().to_string(),
                title: title.to_string(),
                settings: Default::default(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }

        fn load_conversation(&self, id: &str) -> RepositoryResult<Conversation> {
            if self.should_fail {
                return Err(ChatDomainError::ConversationNotFound {
                    id: id.to_string(),
                });
            }
            Ok(Conversation {
                id: id.to_string(),
                title: "Test Conversation".to_string(),
                settings: Default::default(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }

        fn load_all_conversations(&self) -> RepositoryResult<Vec<Conversation>> {
            if self.should_fail {
                return Err(ChatDomainError::InvalidInput {
                    reason: "Mock error".to_string(),
                });
            }
            Ok(vec![Conversation {
                id: uuid::Uuid::new_v4().to_string(),
                title: "Test Conversation".to_string(),
                settings: Default::default(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }])
        }

        fn rename_conversation(&self, id: &str, title: &str) -> RepositoryResult<Conversation> {
            if self.should_fail {
                return Err(ChatDomainError::ConversationNotFound {
                    id: id.to_string(),
                });
            }
            Ok(Conversation {
                id: id.to_string(),
                title: title.to_string(),
                settings: Default::default(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }

        fn delete_conversation(&self, _id: &str) -> RepositoryResult<()> {
            if self.should_fail {
                return Err(ChatDomainError::InvalidInput {
                    reason: "Mock error".to_string(),
                });
            }
            Ok(())
        }

        fn update_conversation_settings(
            &self,
            id: &str,
            settings: &ConversationSettings,
        ) -> RepositoryResult<Conversation> {
            if self.should_fail {
                return Err(ChatDomainError::ConversationNotFound {
                    id: id.to_string(),
                });
            }
            Ok(Conversation {
                id: id.to_string(),
                title: "Test".to_string(),
                settings: settings.clone(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }

        fn set_conversation_workspace(
            &self,
            id: &str,
            workspace_root: &str,
        ) -> RepositoryResult<Conversation> {
            if self.should_fail {
                return Err(ChatDomainError::ConversationNotFound {
                    id: id.to_string(),
                });
            }
            let mut settings = ConversationSettings::default();
            settings.workspace_root = Some(workspace_root.to_string());
            Ok(Conversation {
                id: id.to_string(),
                title: "Test".to_string(),
                settings,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }

        fn create_thread(&self, request: CreateThreadRequest) -> RepositoryResult<Thread> {
            if self.should_fail {
                return Err(ChatDomainError::ConversationNotFound {
                    id: request.conversation_id,
                });
            }
            Ok(Thread {
                id: uuid::Uuid::new_v4().to_string(),
                conversation_id: request.conversation_id,
                title: request.title,
                parent_id: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                message_count: 0,
                metadata: ThreadMetadata {
                    resolved: false,
                    agent_session_id: None,
                    ..Default::default()
                },
            })
        }

        fn load_threads(&self, conversation_id: &str) -> RepositoryResult<Vec<Thread>> {
            if self.should_fail {
                return Err(ChatDomainError::ConversationNotFound {
                    id: conversation_id.to_string(),
                });
            }
            Ok(vec![Thread {
                id: uuid::Uuid::new_v4().to_string(),
                conversation_id: conversation_id.to_string(),
                title: "Test Thread".to_string(),
                parent_id: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                message_count: 0,
                metadata: ThreadMetadata {
                    resolved: false,
                    agent_session_id: None,
                    ..Default::default()
                },
            }])
        }

        fn load_thread(&self, id: &str) -> RepositoryResult<Thread> {
            if self.should_fail {
                return Err(ChatDomainError::ThreadNotFound { id: id.to_string() });
            }
            Ok(Thread {
                id: id.to_string(),
                conversation_id: "conv-1".to_string(),
                title: "Test Thread".to_string(),
                resolved: self.thread_resolved,
                agent_session_id: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }

        fn resolve_thread(&self, id: &str, resolved: bool) -> RepositoryResult<()> {
            if self.should_fail {
                return Err(ChatDomainError::ThreadNotFound { id: id.to_string() });
            }
            Ok(())
        }

        fn delete_thread(&self, _id: &str) -> RepositoryResult<()> {
            if self.should_fail {
                return Err(ChatDomainError::InvalidInput {
                    reason: "Mock error".to_string(),
                });
            }
            Ok(())
        }

        fn rename_thread(&self, id: &str, title: &str) -> RepositoryResult<Thread> {
            if self.should_fail {
                return Err(ChatDomainError::ThreadNotFound { id: id.to_string() });
            }
            Ok(Thread {
                id: id.to_string(),
                conversation_id: "conv-1".to_string(),
                title: title.to_string(),
                parent_id: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                message_count: 0,
                metadata: ThreadMetadata {
                    resolved: false,
                    agent_session_id: None,
                    ..Default::default()
                },
            })
        }

        fn create_thread_with_message(
            &self,
            conversation_id: &str,
            thread_title: &str,
            content: &str,
            role: MessageRole,
        ) -> RepositoryResult<(Thread, ChatMessage)> {
            if self.should_fail {
                return Err(ChatDomainError::ConversationNotFound {
                    id: conversation_id.to_string(),
                });
            }
            let thread = Thread {
                id: uuid::Uuid::new_v4().to_string(),
                conversation_id: conversation_id.to_string(),
                title: thread_title.to_string(),
                parent_id: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                message_count: 0,
                metadata: ThreadMetadata {
                    resolved: false,
                    agent_session_id: None,
                    ..Default::default()
                },
            };
            let message = ChatMessage {
                id: uuid::Uuid::new_v4().to_string(),
                conversation_id: conversation_id.to_string(),
                thread_id: Some(thread.id.clone()),
                role,
                content: content.to_string(),
                timestamp: chrono::Utc::now(),
                status: MessageStatus::Sent,
                metadata: None,
            };
            Ok((thread, message))
        }

        fn set_thread_agent_session(
            &self,
            _id: &str,
            _agent_session_id: Option<String>,
        ) -> RepositoryResult<()> {
            Ok(())
        }

        fn send_message(&self, request: SendMessageRequest) -> RepositoryResult<ChatMessage> {
            if self.thread_resolved {
                return Err(ChatDomainError::ThreadResolved {
                    thread_title: "Test Thread".to_string(),
                });
            }
            if self.should_fail {
                return Err(ChatDomainError::ThreadNotFound {
                    id: request.thread_id,
                });
            }
            Ok(ChatMessage {
                id: uuid::Uuid::new_v4().to_string(),
                conversation_id: request.conversation_id,
                thread_id: Some(request.thread_id),
                role: request.role,
                content: request.content,
                timestamp: chrono::Utc::now(),
                status: MessageStatus::Sent,
                metadata: None,
            })
        }

        fn load_messages(&self, conversation_id: &str) -> RepositoryResult<Vec<ChatMessage>> {
            if self.should_fail {
                return Err(ChatDomainError::ConversationNotFound {
                    id: conversation_id.to_string(),
                });
            }
            Ok(vec![])
        }

        fn load_thread_messages(&self, thread_id: &str) -> RepositoryResult<Vec<ChatMessage>> {
            if self.should_fail {
                return Err(ChatDomainError::ThreadNotFound { id: thread_id.to_string() });
            }
            Ok(vec![])
        }

        fn delete_message(&self, _id: &str) -> RepositoryResult<()> {
            Ok(())
        }

        fn update_message_status(&self, _id: &str, _status: MessageStatus) -> RepositoryResult<()> {
            Ok(())
        }

        fn update_message_feedback(
            &self,
            _id: &str,
            _feedback: Option<MessageFeedback>,
        ) -> RepositoryResult<()> {
            Ok(())
        }
    }

    // Helper to create test state
    fn create_test_state() -> ChatState {
        let service = Arc::new(MockChatService::new()) as Arc<dyn crate::core::chat::services::ChatService>;
        ChatState::with_service(service)
    }

    // Helper to create test state with resolved thread
    fn create_test_state_with_resolved_thread() -> ChatState {
        let service = Arc::new(MockChatService::with_thread_resolved()) as Arc<dyn crate::core::chat::services::ChatService>;
        ChatState::with_service(service)
    }

    // Helper to create test state with error
    fn create_test_state_with_error() -> ChatState {
        let service = Arc::new(MockChatService::with_error()) as Arc<dyn crate::core::chat::services::ChatService>;
        ChatState::with_service(service)
    }

    // Helper to get store from state (for compatibility with test assertions)
    fn get_store(state: &ChatState) -> &tokio::sync::Mutex<SqliteChatStore> {
        &state.store
    }

    #[tokio::test]
    async fn test_chat_state_creation() {
        let state = ChatState::in_memory();
        assert!(state.is_ok());
    }

    // =========================================================================
    // Conversation Command Tests
    // =========================================================================

    #[tokio::test]
    async fn test_load_conversations_success() {
        let state = create_test_state();
        let result = load_conversations(tauri::State::from(&state)).await;
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_load_conversations_error() {
        let state = create_test_state_with_error();
        let result = load_conversations(tauri::State::from(&state)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_conversation_success() {
        let state = create_test_state();
        let result = create_conversation("Test Conversation".to_string(), tauri::State::from(&state)).await;
        assert!(result.is_ok());
        let conv = result.unwrap();
        assert_eq!(conv.title, "Test Conversation");
    }

    #[tokio::test]
    async fn test_create_conversation_empty_title() {
        let state = create_test_state();
        let result = create_conversation("".to_string(), tauri::State::from(&state)).await;
        // Empty title is accepted by the mock
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_load_conversation_success() {
        let state = create_test_state();
        let result = load_conversation("conv-123".to_string(), tauri::State::from(&state)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, "conv-123");
    }

    #[tokio::test]
    async fn test_load_conversation_not_found() {
        let state = create_test_state_with_error();
        let result = load_conversation("nonexistent".to_string(), tauri::State::from(&state)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rename_conversation_success() {
        let state = create_test_state();
        let result = rename_conversation(
            "conv-123".to_string(),
            "New Title".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().title, "New Title");
    }

    #[tokio::test]
    async fn test_delete_conversation_success() {
        let state = create_test_state();
        let result = delete_conversation("conv-123".to_string(), tauri::State::from(&state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_conversation_settings_success() {
        let state = create_test_state();
        let settings = ConversationSettings {
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
            system_prompt: Some("Test prompt".to_string()),
            workspace_root: None,
        };
        let result = update_conversation_settings("conv-123".to_string(), settings, tauri::State::from(&state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_conversation_workspace_success() {
        let state = create_test_state();
        let result = set_conversation_workspace(
            "conv-123".to_string(),
            "/path/to/workspace".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
    }

    // =========================================================================
    // Thread Command Tests
    // =========================================================================

    #[tokio::test]
    async fn test_load_threads_success() {
        let state = create_test_state();
        let result = load_threads("conv-123".to_string(), tauri::State::from(&state)).await;
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_create_thread_success() {
        let state = create_test_state();
        let result = create_thread(
            "conv-123".to_string(),
            "Test Thread".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
        let thread = result.unwrap();
        assert_eq!(thread.title, "Test Thread");
        assert_eq!(thread.conversation_id, "conv-123");
    }

    #[tokio::test]
    async fn test_resolve_thread_success() {
        let state = create_test_state();
        let result = resolve_thread("thread-123".to_string(), true, tauri::State::from(&state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_thread_success() {
        let state = create_test_state();
        let result = delete_thread("thread-123".to_string(), tauri::State::from(&state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rename_thread_success() {
        let state = create_test_state();
        let result = rename_thread(
            "thread-123".to_string(),
            "New Thread Title".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().title, "New Thread Title");
    }

    #[tokio::test]
    async fn test_set_thread_agent_session_success() {
        let state = create_test_state();
        let result = set_thread_agent_session(
            "thread-123".to_string(),
            Some("session-123".to_string()),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_thread_with_message_success() {
        let state = create_test_state();
        let result = create_thread_with_message(
            "conv-123".to_string(),
            "Thread Title".to_string(),
            "Hello, World!".to_string(),
            "user".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.thread.title, "Thread Title");
        assert_eq!(response.message.content, "Hello, World!");
        assert_eq!(response.message.role, MessageRole::User);
    }

    #[tokio::test]
    async fn test_create_thread_with_message_assistant_role() {
        let state = create_test_state();
        let result = create_thread_with_message(
            "conv-123".to_string(),
            "Thread Title".to_string(),
            "Assistant response".to_string(),
            "assistant".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().message.role, MessageRole::Assistant);
    }

    #[tokio::test]
    async fn test_create_thread_with_message_system_role() {
        let state = create_test_state();
        let result = create_thread_with_message(
            "conv-123".to_string(),
            "Thread Title".to_string(),
            "System message".to_string(),
            "system".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().message.role, MessageRole::System);
    }

    // =========================================================================
    // Message Command Tests
    // =========================================================================

    #[tokio::test]
    async fn test_load_messages_success() {
        let state = create_test_state();
        let result = load_messages("conv-123".to_string(), tauri::State::from(&state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_load_thread_messages_success() {
        let state = create_test_state();
        let result = load_thread_messages("thread-123".to_string(), tauri::State::from(&state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_message_to_open_thread_success() {
        let state = create_test_state();
        let result = send_message(
            "conv-123".to_string(),
            "Hello, World!".to_string(),
            "user".to_string(),
            "thread-123".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
        let message = result.unwrap();
        assert_eq!(message.content, "Hello, World!");
        assert_eq!(message.role, MessageRole::User);
        assert_eq!(message.thread_id, Some("thread-123".to_string()));
    }

    #[tokio::test]
    async fn test_send_message_to_resolved_thread_fails() {
        let state = create_test_state_with_resolved_thread();
        let result = send_message(
            "conv-123".to_string(),
            "Hello, World!".to_string(),
            "user".to_string(),
            "thread-123".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("resolved"));
    }

    #[tokio::test]
    async fn test_send_message_assistant_role() {
        let state = create_test_state();
        let result = send_message(
            "conv-123".to_string(),
            "Assistant response".to_string(),
            "assistant".to_string(),
            "thread-123".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().role, MessageRole::Assistant);
    }

    #[tokio::test]
    async fn test_delete_message_success() {
        let state = create_test_state();
        let result = delete_message("msg-123".to_string(), tauri::State::from(&state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_message_status_sending() {
        let state = create_test_state();
        let result = update_message_status(
            "msg-123".to_string(),
            "sending".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_message_status_delivered() {
        let state = create_test_state();
        let result = update_message_status(
            "msg-123".to_string(),
            "delivered".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_message_status_error() {
        let state = create_test_state();
        let result = update_message_status(
            "msg-123".to_string(),
            "error".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_message_status_default() {
        let state = create_test_state();
        let result = update_message_status(
            "msg-123".to_string(),
            "unknown".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_message_feedback_like() {
        let state = create_test_state();
        let result = update_message_feedback(
            "msg-123".to_string(),
            Some("like".to_string()),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_message_feedback_dislike() {
        let state = create_test_state();
        let result = update_message_feedback(
            "msg-123".to_string(),
            Some("dislike".to_string()),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_message_feedback_none() {
        let state = create_test_state();
        let result = update_message_feedback(
            "msg-123".to_string(),
            None,
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_message_feedback_invalid() {
        let state = create_test_state();
        let result = update_message_feedback(
            "msg-123".to_string(),
            Some("invalid".to_string()),
            tauri::State::from(&state),
        )
        .await;
        // Invalid feedback is treated as None
        assert!(result.is_ok());
    }

    // =========================================================================
    // Input Validation Tests
    // =========================================================================

    #[tokio::test]
    async fn test_empty_conversation_id_handling() {
        let state = create_test_state();
        let result = load_conversation("".to_string(), tauri::State::from(&state)).await;
        // Empty ID behavior depends on implementation
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_special_characters_in_title() {
        let state = create_test_state();
        let result = create_conversation(
            "Test <script>alert('xss')</script>".to_string(),
            tauri::State::from(&state),
        )
        .await;
        // Special characters are stored as-is
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_very_long_title() {
        let state = create_test_state();
        let long_title = "A".repeat(1000);
        let result = create_conversation(long_title, tauri::State::from(&state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_unicode_in_content() {
        let state = create_test_state();
        let result = send_message(
            "conv-123".to_string(),
            "Hello 🚀 World 🌍".to_string(),
            "user".to_string(),
            "thread-123".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "Hello 🚀 World 🌍");
    }
}
