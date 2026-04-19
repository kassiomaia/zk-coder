//! Agent subtask planning tool.
//!
//! Allows the LLM to decompose its current task into named subtasks
//! that appear in the Activity section of the agent panel.

use std::path::PathBuf;

use tauri::AppHandle;

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde_json::Value as JsonValue;

/// Tool that allows agents to plan subtasks for the current execution.
pub struct PlanSubtasksTool;

impl Tool for PlanSubtasksTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("agent", "plan_subtasks"),
            description: concat!(
				"Break down your current task into a list of named subtasks. ",
				"These appear in the Activity section so the user can track your progress. ",
				"Call this once early in execution after understanding what needs to be done. ",
				"Each subtask should be a short description of a logical step. ",
				"Example: ['Analyze current code structure', 'Implement new function', 'Write tests']. ",
				"CALL ONLY for multi-step implementation/debugging/refactoring tasks. ",
				"DO NOT CALL for greetings, acknowledgements, small talk, or single-step requests. ",
				"If the user input is conversational (e.g., 'Hi', 'Thanks'), respond without planning. ",
				"For each subtask, specify where its outputs must live via the optional `contract` field ",
				"(e.g. `docs/communications/social/twitter-*.md` for social copy, `reports/*.pdf` for a launch report). ",
				"Writes outside the declared contract are flagged as scope drift and surfaced in the next step's context. ",
				"Declaring a contract is strongly recommended whenever the subtask produces files or documents; ",
				"omit it only for exploratory or read-only subtasks. ",
				"Declare task-level `framing` (role, voice_constraints, optional domain_priors, optional forbidden_patterns) ",
				"for any task where persona or voice matters — communication work, technical writing, launch copy. ",
				"Provide at least two specific voice_constraints; bland one-liners like 'professional and clear' are rejected. ",
				"All forbidden_patterns must be valid regex. Omit `framing` for ordinary code work to inherit the code defaults."
			),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "framing": {
                        "type": "object",
                        "description": "Optional task-level framing (Workstream H.2). Declares the persona, voice, domain priors, and forbidden patterns for this task. Rendered into the system prompt for the lifetime of the task; omit for code work that should inherit the default framing.",
                        "properties": {
                            "role": {
                                "type": "string",
                                "description": "Short role descriptor, e.g. 'senior communication professional', 'technical writer'."
                            },
                            "voiceConstraints": {
                                "type": "array",
                                "minItems": 2,
                                "items": {"type": "string"},
                                "description": "At least 2 opinionated, specific voice constraints. Bland one-line declarations are rejected at planning time."
                            },
                            "domainPriors": {
                                "type": "object",
                                "description": "Optional structured domain priors. v1: a free-form list of hints.",
                                "properties": {
                                    "hints": {
                                        "type": "array",
                                        "items": {"type": "string"}
                                    }
                                }
                            },
                            "forbiddenPatterns": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "Optional regex list. Each pattern must compile; the planner runs validation before persisting."
                            }
                        },
                        "required": ["role", "voiceConstraints"]
                    },
                    "subtasks": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": {
                                    "type": "string",
                                    "description": "Short title for this subtask (2-8 words)"
                                },
                                "description": {
                                    "type": "string",
                                    "description": "Optional longer description of what this subtask involves"
                                },
                                "contract": {
                                    "type": "object",
                                    "description": "Optional expected-output contract. When present, writes outside these paths are flagged as scope drift in the next step's context.",
                                    "properties": {
                                        "expected_artifacts": {
                                            "type": "array",
                                            "description": "Ordered list of artifacts this subtask is expected to produce.",
                                            "items": {
                                                "type": "object",
                                                "properties": {
                                                    "path_pattern": {
                                                        "type": "string",
                                                        "description": "Glob-style pattern (e.g. `docs/social/*.md`, `reports/**/*.pdf`). The runtime matches written paths against this pattern to detect scope drift."
                                                    },
                                                    "kind": {
                                                        "type": "string",
                                                        "enum": ["file", "document", "command", "other"],
                                                        "description": "Artifact kind: `file` for plain files, `document` for generated PDF/DOCX/PPTX, `command` for shell outputs, `other` for anything else."
                                                    }
                                                },
                                                "required": ["path_pattern", "kind"]
                                            },
                                            "minItems": 1
                                        }
                                    },
                                    "required": ["expected_artifacts"]
                                }
                            },
                            "required": ["title"]
                        },
                        "description": "Ordered list of subtasks to plan",
                        "minItems": 1,
                        "maxItems": 10
                    }
                },
                "required": ["subtasks"]
            }),
            requires_workspace: false,
            experimental: false,
            feature_flag: None,
        }
    }

    fn execute(
        &self,
        _context: ToolExecutionContext,
        arguments: JsonValue,
        _app_handle: AppHandle,
        _workspace_root: Option<PathBuf>,
    ) -> ToolFuture {
        Box::pin(async move {
            // Validate subtasks array exists
            let subtasks = arguments
                .get("subtasks")
                .and_then(|v| v.as_array())
                .ok_or_else(|| ToolError::invalid_args("Missing 'subtasks' array parameter"))?;

            if subtasks.is_empty() {
                return Err(ToolError::invalid_args(
                    "'subtasks' array must not be empty",
                ));
            }

            let count = subtasks.len();

            // The actual subtask creation is handled by the agent runtime
            // which intercepts this tool call and creates SubTask objects
            Ok(serde_json::json!({
                "success": true,
                "message": format!("Planned {count} subtasks"),
                "count": count
            }))
        })
    }
}

#[cfg(test)]
mod framing_schema_tests {
    //! Workstream H.2.b — `framing` is declared on the planner schema.
    //!
    //! These tests run unconditionally (not gated on the integration-test
    //! feature) so the schema guarantees are part of the default build.
    use super::*;

    #[test]
    fn schema_exposes_optional_framing_object_with_required_fields() {
        let tool = PlanSubtasksTool;
        let schema = tool.meta().input_schema;
        let framing = &schema["properties"]["framing"];
        assert_eq!(framing["type"], "object");
        let required = framing["required"]
            .as_array()
            .expect("framing.required must be an array");
        let names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
        assert!(names.contains(&"role"));
        assert!(names.contains(&"voiceConstraints"));
        // framing itself is *not* required on the top-level schema.
        let top_required = schema["required"]
            .as_array()
            .expect("top-level required must be an array");
        let top_names: Vec<&str> = top_required.iter().filter_map(|v| v.as_str()).collect();
        assert!(
            !top_names.contains(&"framing"),
            "framing must remain optional at the task level"
        );
    }

    #[test]
    fn schema_enforces_minimum_voice_constraints_count() {
        let tool = PlanSubtasksTool;
        let schema = tool.meta().input_schema;
        let voice = &schema["properties"]["framing"]["properties"]["voiceConstraints"];
        assert_eq!(voice["type"], "array");
        assert_eq!(voice["minItems"], 2);
    }

    #[test]
    fn description_documents_framing_is_for_non_code_tasks() {
        let tool = PlanSubtasksTool;
        let meta = tool.meta();
        assert!(meta.description.contains("framing"));
        assert!(meta.description.contains("voice_constraints"));
        assert!(meta.description.contains("regex"));
    }
}

#[cfg(all(test, feature = "integration-test"))]
mod tests {
    use super::*;

    // === Tool metadata tests ===

    #[test]
    fn test_tool_metadata() {
        let tool = PlanSubtasksTool;
        let meta = tool.meta();

        assert_eq!(meta.id.to_string(), "agent_plan_subtasks");
        assert!(meta.description.contains("subtasks"));
        assert!(meta.description.contains("Activity"));
    }

    #[test]
    fn test_tool_id_format() {
        let tool = PlanSubtasksTool;
        let meta = tool.meta();

        assert_eq!(meta.id.category, "agent");
        assert_eq!(meta.id.name, "plan_subtasks");
        assert_eq!(meta.id.full_name(), "agent_plan_subtasks");
    }

    #[test]
    fn test_input_schema() {
        let tool = PlanSubtasksTool;
        let meta = tool.meta();
        let schema = &meta.input_schema;

        assert_eq!(schema["type"], "object");

        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v.as_str() == Some("subtasks")));

        // Check subtasks array schema
        let subtasks_prop = &schema["properties"]["subtasks"];
        assert_eq!(subtasks_prop["type"], "array");
        assert_eq!(subtasks_prop["minItems"], 1);
        assert_eq!(subtasks_prop["maxItems"], 10);
    }

    #[test]
    fn test_subtask_item_schema() {
        let tool = PlanSubtasksTool;
        let meta = tool.meta();
        let schema = &meta.input_schema;

        let item_schema = &schema["properties"]["subtasks"]["items"];
        assert_eq!(item_schema["type"], "object");

        let required = item_schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("title")));

        assert!(item_schema["properties"]["title"].is_object());
        assert!(item_schema["properties"]["description"].is_object());
    }

    #[test]
    fn test_no_workspace_required() {
        let tool = PlanSubtasksTool;
        let meta = tool.meta();
        assert!(!meta.requires_workspace);
    }

    #[test]
    fn test_not_experimental() {
        let tool = PlanSubtasksTool;
        let meta = tool.meta();
        assert!(!meta.experimental);
    }

    #[test]
    fn test_no_feature_flag() {
        let tool = PlanSubtasksTool;
        let meta = tool.meta();
        assert!(meta.feature_flag.is_none());
    }

    // === Execution tests ===

    #[tokio::test]
    async fn test_execute_with_valid_subtasks() {
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "subtasks": [
                {"title": "First task"},
                {"title": "Second task", "description": "More details"}
            ]
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["success"], true);
        assert_eq!(output["count"], 2);
        assert!(output["message"].as_str().unwrap().contains("2"));
    }

    #[tokio::test]
    async fn test_execute_with_single_subtask() {
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "subtasks": [
                {"title": "Only task"}
            ]
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["count"], 1);
    }

    #[tokio::test]
    async fn test_execute_with_max_subtasks() {
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let mut subtasks = serde_json::json!([]);
        for i in 0..10 {
            subtasks.as_array_mut().unwrap().push(serde_json::json!({
                "title": format!("Task {}", i)
            }));
        }

        let args = serde_json::json!({
            "subtasks": subtasks
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["count"], 10);
    }

    #[tokio::test]
    async fn test_execute_with_subtasks_with_descriptions() {
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "subtasks": [
                {
                    "title": "Analyze code",
                    "description": "Review the existing codebase structure"
                },
                {
                    "title": "Implement feature",
                    "description": "Add the new functionality"
                }
            ]
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap()["count"], 2);
    }

    // === Error handling tests ===

    #[tokio::test]
    async fn test_execute_missing_subtasks() {
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({});

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("subtasks"));
    }

    #[tokio::test]
    async fn test_execute_empty_subtasks_array() {
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "subtasks": []
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[tokio::test]
    async fn test_execute_subtasks_not_an_array() {
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "subtasks": "not an array"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_subtasks_with_missing_titles() {
        // The tool doesn't validate individual subtask structure,
        // only that the array exists and is not empty
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "subtasks": [
                {"description": "No title"}
            ]
        });

        // Should succeed - validation of subtask structure is up to the runtime
        let result = tool.execute(context, args, app_handle, None).await;
        assert!(result.is_ok());
    }

    // === Output validation tests ===

    #[tokio::test]
    async fn test_output_structure() {
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "subtasks": [
                {"title": "Task 1"}
            ]
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();

        assert_eq!(output["success"], true);
        assert!(output["message"].is_string());
        assert!(output["count"].is_number());
    }

    #[tokio::test]
    async fn test_output_message_format() {
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "subtasks": [
                {"title": "Task 1"},
                {"title": "Task 2"},
                {"title": "Task 3"}
            ]
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();

        let message = output["message"].as_str().unwrap();
        assert!(message.contains("Planned"));
        assert!(message.contains("3"));
        assert!(message.contains("subtasks"));
    }

    #[tokio::test]
    async fn test_output_is_json_serializable() {
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "subtasks": [
                {"title": "Task 1"}
            ]
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();

        // Verify we can serialize back to JSON string
        let json_string = serde_json::to_string(&output);
        assert!(json_string.is_ok());
    }

    // === Context tests ===

    #[tokio::test]
    async fn test_execute_with_thread_context() {
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation").with_thread("thread-123");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "subtasks": [
                {"title": "Task 1"}
            ]
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_session_context() {
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation").with_session("session-456");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "subtasks": [
                {"title": "Task 1"}
            ]
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_without_workspace() {
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "subtasks": [
                {"title": "Task 1"}
            ]
        });

        // Should work without workspace
        let result = tool.execute(context, args, app_handle, None).await;
        assert!(result.is_ok());
    }

    // === Edge case tests ===

    #[tokio::test]
    async fn test_execute_with_extra_fields_ignored() {
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "subtasks": [
                {"title": "Task 1"}
            ],
            "unexpected": "field"
        });

        // Extra fields are ignored by the execute function
        let result = tool.execute(context, args, app_handle, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_subtask_with_empty_title() {
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "subtasks": [
                {"title": ""}
            ]
        });

        // The tool doesn't validate title content, only array presence
        let result = tool.execute(context, args, app_handle, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["count"], 1);
    }

    #[tokio::test]
    async fn test_execute_with_unicode_titles() {
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "subtasks": [
                {"title": "Analyze code 分析"},
                {"title": "Test tests 测试 🧪"}
            ]
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap()["count"], 2);
    }

    #[tokio::test]
    async fn test_execute_with_very_long_title() {
        let tool = PlanSubtasksTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let long_title = "A".repeat(1000);
        let args = serde_json::json!({
            "subtasks": [
                {"title": long_title}
            ]
        });

        let result = tool.execute(context, args, app_handle, None).await;

        // Should still work - no length validation on titles
        assert!(result.is_ok());
    }
}
