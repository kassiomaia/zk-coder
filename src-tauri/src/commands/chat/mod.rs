//! Chat Tauri IPC commands.
//!
//! Thin adapter layer that delegates to the ChatService.

pub mod conversations;
pub mod messages;
mod state;
#[cfg(all(test, feature = "integration-test"))]
mod tests;
pub mod threads;

pub use conversations::{
    create_conversation, delete_conversation, load_conversation, load_conversations,
    rename_conversation, set_conversation_workspace, update_conversation_settings,
};
pub use messages::{
    delete_message, load_messages, load_thread_messages, send_message, update_message_feedback,
    update_message_status,
};
pub use state::ChatState;
pub use threads::{
    create_thread, create_thread_with_message, delete_thread, load_threads, rename_thread,
    resolve_thread, set_thread_agent_session, ThreadWithMessageResponse,
};
