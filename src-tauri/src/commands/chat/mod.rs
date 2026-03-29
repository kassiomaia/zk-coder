//! Chat Tauri IPC commands.
//!
//! Thin adapter layer that delegates to the ChatService.

/// Re-exports for conversation-related commands.
pub mod conversations;
/// Re-exports for message-related commands.
pub mod messages;
mod state;
#[cfg(all(test, feature = "integration-test"))]
mod tests;
/// Re-exports for thread-related commands.
pub mod threads;

// Re-exports from conversations module
pub use conversations::{
    create_conversation, delete_conversation, load_conversation, load_conversations,
    rename_conversation, set_conversation_workspace, update_conversation_settings,
};
// Re-exports from messages module
pub use messages::{
    delete_message, load_messages, load_thread_messages, send_message, update_message_feedback,
    update_message_status,
};
// Re-exports from state module
pub use state::ChatState;
// Re-exports from threads module
pub use threads::{
    create_thread, create_thread_with_message, delete_thread, load_threads, rename_thread,
    resolve_thread, set_thread_agent_session, ThreadWithMessageResponse,
};
