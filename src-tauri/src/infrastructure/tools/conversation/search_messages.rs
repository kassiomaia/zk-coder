//! Search conversation messages tool.

use crate::commands::chat::ChatState;
use crate::infrastructure::tools::constants::{
    DEFAULT_MESSAGE_SEARCH_LIMIT, MAX_MESSAGE_SEARCH_LIMIT,
};
use crate::infrastructure::tools::{
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

const fn default_limit() -> usize {
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
            let args: SearchMessagesArgs = crate::infrastructure::tools::error::parse_args(
                "conversation_search_messages",
                arguments,
            )?;

            let needle = args.query.trim().to_ascii_lowercase();
            if needle.is_empty() {
                return Err(ToolError::invalid_args("Query cannot be empty"));
            }

            let limit = args.limit.clamp(1, MAX_MESSAGE_SEARCH_LIMIT);

            let chat_state = app_handle.state::<ChatState>();
            let history = chat_state
                .service
                .load_messages(&context.conversation_id)
                .map_err(|e| ToolError::failed(format!("Failed to load messages: {e}")))?;

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

#[cfg(all(test, feature = "integration-test"))]
mod tests {
    use super::*;

    // === Tool metadata tests ===

    #[test]
    fn test_tool_metadata() {
        let tool = SearchMessagesTool;
        let meta = tool.meta();

        assert_eq!(meta.id.to_string(), "conversation_search_messages");
        assert!(meta.description.contains("search"));
        assert!(!meta.requires_workspace);
        assert!(!meta.experimental);
        assert!(meta.feature_flag.is_none());
    }

    #[test]
    fn test_tool_id_format() {
        let tool = SearchMessagesTool;
        let meta = tool.meta();

        assert_eq!(meta.id.category, "conversation");
        assert_eq!(meta.id.name, "search_messages");
        assert_eq!(meta.id.full_name(), "conversation_search_messages");
    }

    #[test]
    fn test_input_schema() {
        let tool = SearchMessagesTool;
        let meta = tool.meta();
        let schema = &meta.input_schema;

        assert_eq!(schema["type"], "object");
        assert!(schema["additionalProperties"] == false);

        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("query")));

        // Check limit constraints
        let limit_prop = &schema["properties"]["limit"];
        assert_eq!(limit_prop["type"], "integer");
        assert_eq!(limit_prop["minimum"], 1);
        assert_eq!(limit_prop["maximum"], MAX_MESSAGE_SEARCH_LIMIT);
    }

    // === Arguments parsing tests ===

    #[test]
    fn test_parse_args_valid() {
        let args = serde_json::json!({
            "query": "test search",
            "limit": 10
        });

        let parsed: SearchMessagesArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.query, "test search");
        assert_eq!(parsed.limit, 10);
    }

    #[test]
    fn test_parse_args_defaults() {
        let args = serde_json::json!({
            "query": "test"
        });

        let parsed: SearchMessagesArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.query, "test");
        assert_eq!(parsed.limit, DEFAULT_MESSAGE_SEARCH_LIMIT);
    }

    #[test]
    fn test_parse_args_missing_query() {
        let args = serde_json::json!({
            "limit": 10
        });

        let parsed: Result<SearchMessagesArgs, _> = serde_json::from_value(args);
        assert!(parsed.is_err());
    }

    #[test]
    fn test_default_limit() {
        assert_eq!(default_limit(), DEFAULT_MESSAGE_SEARCH_LIMIT);
    }

    // === MessageMatch tests ===

    #[test]
    fn test_message_match_fields() {
        let msg_match = MessageMatch {
            id: "msg-123".to_string(),
            role: "user".to_string(),
            thread_id: Some("thread-456".to_string()),
            timestamp: 1234567890,
            content: "Hello, world!".to_string(),
        };

        assert_eq!(msg_match.id, "msg-123");
        assert_eq!(msg_match.role, "user");
        assert_eq!(msg_match.thread_id, Some("thread-456".to_string()));
        assert_eq!(msg_match.timestamp, 1234567890);
        assert_eq!(msg_match.content, "Hello, world!");
    }

    #[test]
    fn test_message_match_serialization() {
        let msg_match = MessageMatch {
            id: "msg-123".to_string(),
            role: "assistant".to_string(),
            thread_id: None,
            timestamp: 1234567890,
            content: "Response".to_string(),
        };

        let json = serde_json::to_value(&msg_match).unwrap();
        assert_eq!(json["id"], "msg-123");
        assert_eq!(json["role"], "assistant");
        assert!(json["thread_id"].is_null());
        assert_eq!(json["timestamp"], 1234567890);
        assert_eq!(json["content"], "Response");
    }

    // === SearchMessagesOutput tests ===

    #[test]
    fn test_search_output_fields() {
        let output = SearchMessagesOutput {
            conversation_id: "conv-123".to_string(),
            query: "test".to_string(),
            count: 2,
            matches: vec![MessageMatch {
                id: "msg-1".to_string(),
                role: "user".to_string(),
                thread_id: None,
                timestamp: 1000,
                content: "test message".to_string(),
            }],
        };

        assert_eq!(output.conversation_id, "conv-123");
        assert_eq!(output.query, "test");
        assert_eq!(output.count, 2);
        assert_eq!(output.matches.len(), 1);
    }

    #[test]
    fn test_search_output_empty_matches() {
        let output = SearchMessagesOutput {
            conversation_id: "conv-123".to_string(),
            query: "nonexistent".to_string(),
            count: 0,
            matches: vec![],
        };

        assert_eq!(output.count, 0);
        assert!(output.matches.is_empty());
    }

    #[test]
    fn test_search_output_serialization() {
        let output = SearchMessagesOutput {
            conversation_id: "conv-123".to_string(),
            query: "test".to_string(),
            count: 1,
            matches: vec![MessageMatch {
                id: "msg-1".to_string(),
                role: "user".to_string(),
                thread_id: None,
                timestamp: 1000,
                content: "test".to_string(),
            }],
        };

        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["conversation_id"], "conv-123");
        assert_eq!(json["query"], "test");
        assert_eq!(json["count"], 1);
        assert!(json["matches"].is_array());
    }

    // === Input validation tests ===

    #[tokio::test]
    async fn test_execute_empty_query_fails() {
        let tool = SearchMessagesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "query": "   "
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[tokio::test]
    async fn test_execute_missing_query_fails() {
        let tool = SearchMessagesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({});

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_without_workspace() {
        // This tool doesn't require workspace
        let tool = SearchMessagesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "query": "test"
        });

        // Will fail on ChatState but not on workspace requirement
        let result = tool.execute(context, args, app_handle, None).await;

        // Should fail because ChatState is not available in test
        assert!(result.is_err());
    }

    // === Case-insensitive search tests ===

    #[test]
    fn test_query_is_lowercased() {
        let needle = "HeLLo WoRLd".trim().to_ascii_lowercase();
        assert_eq!(needle, "hello world");
    }

    #[test]
    fn test_search_is_case_insensitive() {
        let content = "Hello World".to_string();
        let query = "hello world".to_ascii_lowercase();
        assert!(content.to_ascii_lowercase().contains(&query));
    }

    #[test]
    fn test_search_with_unicode() {
        let content = "Hello 世界".to_string();
        let query = "世界".to_ascii_lowercase();
        // ASCII lowercase doesn't affect unicode, but this should still work
        let content_lower = content.to_ascii_lowercase();
        assert!(!content_lower.contains(&query)); // This won't match as-is
                                                  // But the content still contains the original unicode
        assert!(content.contains("世界"));
    }

    // === Limit clamping tests ===

    #[test]
    fn test_limit_is_clamped_to_max() {
        let limit = 1000;
        let clamped = limit.clamp(1, MAX_MESSAGE_SEARCH_LIMIT);
        assert_eq!(clamped, MAX_MESSAGE_SEARCH_LIMIT);
    }

    #[test]
    fn test_limit_is_clamped_to_min() {
        let limit = 0;
        let clamped = limit.clamp(1, MAX_MESSAGE_SEARCH_LIMIT);
        assert_eq!(clamped, 1);
    }

    #[test]
    fn test_negative_limit_is_clamped() {
        let limit = -10;
        let clamped = (limit as isize).clamp(1, MAX_MESSAGE_SEARCH_LIMIT as isize);
        assert_eq!(clamped, 1);
    }

    // === Edge case tests ===

    #[test]
    fn test_query_with_special_characters() {
        let query = "test@#$%^&*()";
        let trimmed = query.trim();
        assert_eq!(trimmed, "test@#$%^&*()");
    }

    #[test]
    fn test_query_with_newlines() {
        let query = "test\n\nquery";
        let trimmed = query.trim();
        assert!(!trimmed.contains('\n'));
        assert_eq!(trimmed, "test\n\nquery"); // trim only removes leading/trailing
    }

    #[test]
    fn test_message_match_with_all_roles() {
        for role in ["user", "assistant", "system"] {
            let msg_match = MessageMatch {
                id: "msg-123".to_string(),
                role: role.to_string(),
                thread_id: None,
                timestamp: 1000,
                content: "test".to_string(),
            };
            assert_eq!(msg_match.role, role);
        }
    }

    // === Constants tests ===

    #[test]
    fn test_default_message_search_limit() {
        assert_eq!(DEFAULT_MESSAGE_SEARCH_LIMIT, 20);
    }

    #[test]
    fn test_max_message_search_limit() {
        assert_eq!(MAX_MESSAGE_SEARCH_LIMIT, 100);
    }
}
