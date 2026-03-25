//! Search conversation messages tool.

use crate::commands::chat::ChatState;
use crate::core::chat::repositories::MessageRepository;
use crate::tools::constants::{DEFAULT_MESSAGE_SEARCH_LIMIT, MAX_MESSAGE_SEARCH_LIMIT};
use crate::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tauri::{AppHandle, Manager};

/// Arguments for searching conversation messages.
#[derive(Debug, Deserialize)]
pub struct SearchMessagesArgs {
    /// Search query string.
    pub query: String,
    /// Maximum number of matches to return (default: 20, max: 100).
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    DEFAULT_MESSAGE_SEARCH_LIMIT
}

/// Individual message match result.
#[derive(Debug, Serialize)]
pub struct MessageMatch {
    /// Message ID.
    pub id: String,
    /// Message role (user/assistant/system).
    pub role: String,
    /// Associated thread ID (if any).
    pub thread_id: Option<String>,
    /// Message timestamp.
    pub timestamp: i64,
    /// Message content.
    pub content: String,
}

/// Output from searching messages.
#[derive(Debug, Serialize)]
pub struct SearchMessagesOutput {
    /// Conversation ID.
    pub conversation_id: String,
    /// Search query.
    pub query: String,
    /// Number of matches found.
    pub count: usize,
    /// Matching messages.
    pub matches: Vec<MessageMatch>,
}

/// Tool to search conversation messages for a keyword or phrase (case-insensitive).
pub struct SearchMessagesTool;

impl Tool for SearchMessagesTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("conversation", "search_messages"),
            description: "Search existing conversation messages for a query (case-insensitive).",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query string"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum matches to return (default: 20, max: 100)",
                        "minimum": 1,
                        "maximum": MAX_MESSAGE_SEARCH_LIMIT
                    }
                },
                "required": ["query"]
            }),
            requires_workspace: false,
            experimental: false,
            feature_flag: None,
        }
    }

    fn execute(
        &self,
        context: ToolExecutionContext,
        arguments: JsonValue,
        app_handle: AppHandle,
        _workspace_root: Option<std::path::PathBuf>,
    ) -> ToolFuture {
        Box::pin(async move {
            let args: SearchMessagesArgs =
                crate::tools::error::parse_args("conversation_search_messages", arguments)?;

            let needle = args.query.trim().to_ascii_lowercase();
            if needle.is_empty() {
                return Err(ToolError::invalid_args("Query cannot be empty"));
            }

            let limit = args.limit.clamp(1, MAX_MESSAGE_SEARCH_LIMIT);

            let chat_state = app_handle.state::<ChatState>();
            let chat_store = chat_state.store.lock().await;

            let history = chat_store
                .load_messages(&context.conversation_id)
                .map_err(|e| ToolError::failed(format!("Failed to load messages: {}", e)))?;

            let mut matches = Vec::new();
            for message in history {
                if matches.len() >= limit {
                    break;
                }
                if !message.content.to_ascii_lowercase().contains(&needle) {
                    continue;
                }

                matches.push(MessageMatch {
                    id: message.id,
                    role: message.role.as_str().to_string(),
                    thread_id: message.thread_id,
                    timestamp: message.timestamp.timestamp(),
                    content: message.content,
                });
            }

            let output = SearchMessagesOutput {
                conversation_id: context.conversation_id,
                query: args.query,
                count: matches.len(),
                matches,
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}
