use log::warn;

use crate::core::chat::services::ChatService;
use crate::infrastructure::agent::runtime::permissions::YoloModeProvider;

/// Thread-backed YOLO mode provider using the chat service.
pub struct ChatYoloModeProvider<'a> {
    chat_service: &'a dyn ChatService,
}

impl<'a> ChatYoloModeProvider<'a> {
    /// Creates a new YOLO mode provider backed by the given chat service.
    #[must_use]
    pub const fn new(chat_service: &'a dyn ChatService) -> Self {
        Self { chat_service }
    }
}

impl YoloModeProvider for ChatYoloModeProvider<'_> {
    fn is_yolo_enabled(&self, thread_id: &str) -> bool {
        match self.chat_service.load_thread(thread_id) {
            Ok(thread) => thread.metadata.yolo_mode,
            Err(error) => {
                warn!("[AgentRuntime] Failed to load thread {thread_id} for YOLO check: {error}");
                false
            }
        }
    }
}
