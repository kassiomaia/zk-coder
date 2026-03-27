//! Conversation and memory tools.

mod search_messages;

use crate::infrastructure::tools::{Tool, ToolCategory};
pub use search_messages::SearchMessagesTool;

/// Conversation tools category for searching and managing conversation data.
pub struct ConversationCategory;

impl ToolCategory for ConversationCategory {
    fn name(&self) -> &'static str {
        "conversation"
    }

    fn description(&self) -> &'static str {
        "Tools for searching and managing conversation data"
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![Box::new(SearchMessagesTool)]
    }
}
