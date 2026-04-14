//! Task planning service for single-shot LLM-based task planning.
//!
//! This service handles:
//! - Single-shot planning at task creation (title, description, subtasks)
//! - Replanning for interrupted tasks (review progress, update plan)

use crate::core::agent::entities::{AgentTask, TaskMode, TaskStatus};
use crate::core::llm::entities::{ChatMessage, ChatRequest, MessageContent, MessageRole};
use crate::infrastructure::agent::runtime::client::create_llm_client;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

/// Result of a single-shot planning operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    /// Generated task title (concise, clear)
    pub title: String,
    /// Generated task description (one-sentence summary)
    pub description: String,
    /// Whether this task requires subtask planning
    pub is_non_trivial: bool,
    /// Planned subtasks (if non-trivial)
    pub subtasks: Vec<SubtaskPlan>,
}

/// A single subtask in the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskPlan {
    /// Subtask title
    pub title: String,
    /// Optional subtask description
    pub description: Option<String>,
    /// Execution order
    pub order: i32,
}

/// Result of a replan operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplanOutput {
    /// The decision made (replan, abort, continue)
    pub decision: ReplanAction,
    /// Updated plan (if replanning)
    pub updated_plan: Option<TaskPlan>,
    /// Indices of completed subtasks
    pub completed_indices: Vec<usize>,
    /// User-friendly summary message
    pub user_message: String,
    /// Next step index to resume from
    pub next_step_index: Option<usize>,
}

/// Action to take when reviewing an interrupted task.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReplanAction {
    /// Update and continue with the current task
    Replan,
    /// Abandon current task and start a new one
    Abort,
    /// Resume without changes
    Continue,
}

/// Strips markdown code block wrappers from LLM responses.
///
/// LLMs often wrap JSON in ```json ... ``` blocks even when instructed not to.
/// This function extracts the actual content from within such blocks.
fn strip_markdown_code_block(content: &str) -> String {
    let content = content.trim();

    // If content is empty, return early
    if content.is_empty() {
        warn!("[Planner] strip_markdown_code_block called with empty content");
        return String::new();
    }

    // Check for ```json or ``` at the start
    if content.starts_with("```") {
        // Find the end of the opening fence line
        let after_opening_fence = if let Some(newline_pos) = content.find('\n') {
            &content[newline_pos + 1..]
        } else {
            // No newline after opening fence, treat entire thing as fence
            warn!("[Planner] Markdown fence found but no newline, returning empty");
            return String::new();
        };

        // Find the closing fence (```)
        let closing_fence = "\n```";
        if let Some(closing_pos) = after_opening_fence.find(closing_fence) {
            let result = after_opening_fence[..closing_pos].trim().to_string();
            // Safety check: if we get empty result but had content, log and return original
            if result.is_empty() && !after_opening_fence.trim().is_empty() {
                warn!("[Planner] Markdown stripping with newline fence resulted in empty, returning original content");
                return after_opening_fence.to_string();
            }
            return result;
        }

        // Also check for closing fence without leading newline
        let closing_fence_no_nl = "```";
        if let Some(closing_pos) = after_opening_fence.find(closing_fence_no_nl) {
            let result = after_opening_fence[..closing_pos].trim().to_string();
            // Safety check: if we get empty result but had content, log and return original
            if result.is_empty() && !after_opening_fence.trim().is_empty() {
                warn!("[Planner] Markdown stripping without newline fence resulted in empty, returning original content");
                return after_opening_fence.to_string();
            }
            return result;
        }

        // No closing fence found, return everything after opening fence
        let result = after_opening_fence.trim().to_string();
        if result.is_empty() && !after_opening_fence.trim().is_empty() {
            warn!("[Planner] No closing fence found but stripping resulted in empty, returning original content");
            return after_opening_fence.to_string();
        }
        return result;
    }

    // Check for closing fence at the end (even without opening fence)
    if let Some(fence_pos) = content.rfind("\n```") {
        let possible_content = &content[..fence_pos];
        // Only strip if there's meaningful content before it
        let trimmed = possible_content.trim();
        if !trimmed.is_empty() && !trimmed.starts_with("```") {
            info!("[Planner] Stripped closing fence from end of content");
            return trimmed.to_string();
        }
    }

    // No markdown wrapper found, return original
    info!("[Planner] No markdown wrapper found, returning original content");
    content.to_string()
}

/// Removes leading `<think>...</think>` blocks from an LLM response.
///
/// Some providers prepend reasoning wrapped in XML-like tags even when the user
/// requested plain JSON output. For planning, that reasoning is transport noise.
fn strip_leading_think_blocks(content: &str) -> String {
    let mut remaining = content.trim();

    loop {
        if !remaining.starts_with("<think>") {
            break;
        }

        let Some(end_tag_index) = remaining.find("</think>") else {
            warn!("[Planner] Unterminated <think> block found in planner response");
            break;
        };

        remaining = remaining[end_tag_index + "</think>".len()..].trim_start();
    }

    remaining.to_string()
}

/// Extracts the first complete top-level JSON object from a response string.
///
/// This tolerates explanatory text before or after the JSON body while avoiding
/// partial-object parsing. It only returns object-shaped JSON because planner
/// responses are required to be objects.
fn extract_first_json_object(content: &str) -> Option<String> {
    let bytes = content.as_bytes();
    let mut start_index = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (index, byte) in bytes.iter().enumerate() {
        match byte {
            b'\\' if in_string => {
                escape = !escape;
            }
            b'"' if !escape => {
                in_string = !in_string;
            }
            _ => {
                escape = false;

                if in_string {
                    continue;
                }

                match byte {
                    b'{' => {
                        if start_index.is_none() {
                            start_index = Some(index);
                        }
                        depth += 1;
                    }
                    b'}' if depth > 0 => {
                        depth -= 1;
                        if depth == 0 {
                            let start = start_index?;
                            return Some(content[start..=index].trim().to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    None
}

/// Normalizes an LLM response into the JSON object expected by the planner.
///
/// The planner accepts provider-specific wrappers such as `<think>` blocks,
/// fenced code blocks, or prose surrounding the JSON payload.
fn normalize_planning_json(content: &str) -> Result<String, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("LLM returned empty response. This may indicate a configuration issue with your LLM provider.".to_string());
    }

    let without_think = strip_leading_think_blocks(trimmed);
    let without_markdown = strip_markdown_code_block(&without_think);

    if without_markdown.trim().is_empty() {
        return Err(format!(
            "LLM returned empty response after normalization. Raw response was: '{}'",
            content
        ));
    }

    if let Some(json) = extract_first_json_object(&without_markdown) {
        return Ok(json);
    }

    Err(format!(
        "Could not locate a JSON object in planner response. Normalized response was: '{}'",
        without_markdown
    ))
}

/// Create a task plan using single-shot LLM planning.
///
/// This generates a title, description, and optionally subtasks all in one LLM call.
pub async fn plan_new_task(
    app_handle: &AppHandle,
    user_message: &str,
    model_id: Option<&str>,
) -> Result<TaskPlan, String> {
    let (client, actual_model) =
        create_llm_client(app_handle, model_id.map(std::string::ToString::to_string))
            .await
            .map_err(|e| format!("Failed to create LLM client for planning: {e}"))?;

    // Build the planning prompt
    let system_prompt = build_planning_system_prompt();

    let user_prompt =
        format!("User Request: {user_message}\n\nCreate a task plan for this request.");

    let request = ChatRequest::new(
        actual_model,
        vec![
            ChatMessage {
                role: MessageRole::System,
                content: MessageContent::Text(system_prompt),
                tool_calls: Vec::new(),
                tool_result: None,
            },
            ChatMessage {
                role: MessageRole::User,
                content: MessageContent::Text(user_prompt),
                tool_calls: Vec::new(),
                tool_result: None,
            },
        ],
    )
    .with_max_tokens(2000)
    .with_temperature(0.3);

    // Execute LLM call
    let response = client
        .chat(request)
        .await
        .map_err(|e| format!("LLM planning failed: {e}"))?;

    // Log raw response for debugging
    info!("[Planner] Raw LLM response: {}", &response.content);

    let content = normalize_planning_json(&response.content)?;

    // Log stripped content for debugging
    info!("[Planner] Stripped content: {}", &content);

    // Parse JSON response
    let plan: TaskPlan = serde_json::from_str(&content).map_err(|e| {
        format!("Failed to parse planning response as JSON: {e}\n\nResponse was: {content}")
    })?;

    Ok(plan)
}

/// Review an interrupted task and decide on next action.
///
/// This is called when a user sends a message while a task is incomplete.
pub async fn review_and_replan(
    app_handle: &AppHandle,
    current_task: &AgentTask,
    subtasks: &[AgentTask],
    user_message: &str,
    progress_summary: &str,
    model_id: Option<&str>,
) -> Result<ReplanOutput, String> {
    let (client, actual_model) =
        create_llm_client(app_handle, model_id.map(std::string::ToString::to_string))
            .await
            .map_err(|e| format!("Failed to create LLM client for replanning: {e}"))?;

    // Build current plan summary
    let current_plan = TaskPlan {
        title: current_task.title.clone(),
        description: current_task.description.clone().unwrap_or_default(),
        is_non_trivial: !subtasks.is_empty(),
        subtasks: subtasks
            .iter()
            .map(|st| SubtaskPlan {
                title: st.title.clone(),
                description: st.description.clone(),
                order: st.order,
            })
            .collect(),
    };

    let system_prompt = build_replan_system_prompt();

    let user_prompt = format!(
        "Current Plan:\n{}\n\nProgress Summary:\n{}\n\nUser's New Request:\n{}\n\nAnalyze and decide what to do next.",
        serde_json::to_string(&current_plan).unwrap_or_default(),
        progress_summary,
        user_message
    );

    let request = ChatRequest::new(
        actual_model,
        vec![
            ChatMessage {
                role: MessageRole::System,
                content: MessageContent::Text(system_prompt),
                tool_calls: Vec::new(),
                tool_result: None,
            },
            ChatMessage {
                role: MessageRole::User,
                content: MessageContent::Text(user_prompt),
                tool_calls: Vec::new(),
                tool_result: None,
            },
        ],
    );

    let response = client
        .chat(request)
        .await
        .map_err(|e| format!("LLM replan failed: {e}"))?;

    let content = normalize_planning_json(&response.content)?;

    // Parse JSON response
    let output: ReplanOutput = serde_json::from_str(&content).map_err(|e| {
        format!("Failed to parse replan response as JSON: {e}\n\nResponse was: {content}")
    })?;

    Ok(output)
}

/// Build the system prompt for single-shot task planning.
#[must_use]
pub fn build_planning_system_prompt() -> String {
    r#"You are a task planning assistant. Your job is to analyze a user's request and create a clear, actionable plan.

Generate a JSON response with this exact structure:
{
  "title": "Brief, clear task title (3-8 words)",
  "description": "One-sentence summary of what will be done",
  "is_non_trivial": true or false,
  "subtasks": [
    {
      "title": "Step 1: ...",
      "description": "What this step accomplishes",
      "order": 0
    }
  ]
}

Rules:
- Title should be concise and descriptive (e.g., "Codebase investigation and improvement analysis")
- Description should be one clear sentence
- Set is_non_trivial to false for: greetings, acknowledgements, simple questions, single file reads
- For non-trivial tasks, create 3-8 subtasks
- Each subtask should be independently achievable
- Order subtasks logically (0, 1, 2, ...)
- Keep subtask titles actionable (start with verbs)

Return ONLY the JSON object, nothing else. Do not include reasoning tags like <think> or markdown fences."#
        .to_string()
}

/// Build the system prompt for replanning an interrupted task.
#[must_use]
pub fn build_replan_system_prompt() -> String {
    r#"You are reviewing an interrupted agent task. The user has sent a new message while a task was still in progress.

Your job is to:
1. Understand what was being worked on
2. Review what progress was made
3. Understand the user's new request
4. Decide whether to continue, replan, or abort the current task

Generate a JSON response with this exact structure:
{
  "decision": "REPLAN" | "ABORT" | "CONTINUE",
  "completed_indices": [0, 1, ...],
  "updated_plan": {
    "title": "...",
    "description": "...",
    "is_non_trivial": true,
    "subtasks": [...]
  },
  "user_message": "Friendly summary for the user",
  "next_step_index": 2
}

Decision logic:
- ABORT: User's request is completely unrelated (e.g., was coding, now asking about cooking)
- REPLAN: User is still interested in the same topic but with new context/requirements
- CONTINUE: User's message is a minor clarification or confirmation

For REPLAN:
- Include completed_indices (which subtasks are already done)
- Create updated_plan with remaining work as new subtasks
- Set next_step_index to where to resume

For ABORT or CONTINUE:
- updated_plan can be null
- Set next_step_index to null

Return ONLY the JSON object, nothing else. Do not include reasoning tags like <think> or markdown fences."#
        .to_string()
}

/// Convert a `SubtaskPlan` to an `AgentTask` entity.
#[must_use]
pub fn subtask_plan_to_agent_task(
    subtask_plan: &SubtaskPlan,
    session_id: &str,
    parent_task_id: &str,
) -> AgentTask {
    AgentTask {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        parent_task_id: Some(parent_task_id.to_string()),
        title: subtask_plan.title.clone(),
        description: subtask_plan.description.clone(),
        status: TaskStatus::Pending,
        order: subtask_plan.order,
        mode: Some(TaskMode::FullAccess),
        needs_review: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::llm::mock::MockLlmClient;
    use crate::infrastructure::llm::LlmClient;

    // ========================================================================
    // Serialization Tests (existing)
    // ========================================================================

    #[test]
    fn test_task_plan_serialization() {
        let plan = TaskPlan {
            title: "Test Task".to_string(),
            description: "A test task".to_string(),
            is_non_trivial: true,
            subtasks: vec![SubtaskPlan {
                title: "Step 1".to_string(),
                description: Some("First step".to_string()),
                order: 0,
            }],
        };

        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("Test Task"));
        assert!(json.contains("Step 1"));
    }

    #[test]
    fn test_replan_output_serialization() {
        let output = ReplanOutput {
            decision: ReplanAction::Replan,
            updated_plan: Some(TaskPlan {
                title: "Updated Task".to_string(),
                description: "Updated".to_string(),
                is_non_trivial: true,
                subtasks: vec![],
            }),
            completed_indices: vec![0, 1],
            user_message: "Continuing work".to_string(),
            next_step_index: Some(2),
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("replan"));
        assert!(json.contains("Continuing work"));
    }

    #[test]
    fn test_replan_action_serialization() {
        let action = ReplanAction::Replan;
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("replan"));
    }

    #[test]
    fn test_subtask_plan_to_agent_task() {
        let plan = SubtaskPlan {
            title: "Test subtask".to_string(),
            description: Some("Test description".to_string()),
            order: 0,
        };

        let task = subtask_plan_to_agent_task(&plan, "session-123", "task-456");

        assert_eq!(task.title, "Test subtask");
        assert_eq!(task.description, Some("Test description".to_string()));
        assert_eq!(task.session_id, "session-123");
        assert_eq!(task.parent_task_id, Some("task-456".to_string()));
        assert_eq!(task.order, 0);
        assert_eq!(task.status, TaskStatus::Pending);
    }

    // ========================================================================
    // Response Normalization Tests
    // ========================================================================

    #[test]
    fn test_strip_markdown_code_block_with_json_wrapper() {
        // Test the exact format the LLM returned
        let wrapped = r#"```json
{
  "title": "Test",
  "description": "Test description",
  "is_non_trivial": false,
  "subtasks": []
}
```"#;

        let stripped = strip_markdown_code_block(wrapped);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();

        assert_eq!(parsed["title"], "Test");
        assert!(!stripped.starts_with("```"));
        assert!(!stripped.contains("json"));
    }

    #[test]
    fn test_strip_markdown_code_block_with_plain_wrapper() {
        let wrapped = r#"```
{
  "title": "Test"
}
```"#;

        let stripped = strip_markdown_code_block(wrapped);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();

        assert_eq!(parsed["title"], "Test");
        assert!(!stripped.starts_with("```"));
    }

    #[test]
    fn test_strip_markdown_code_block_plain_json() {
        // Plain JSON should pass through unchanged
        let plain = r#"{"title": "Test", "description": "Test"}"#;

        let stripped = strip_markdown_code_block(plain);

        assert_eq!(stripped, plain);
    }

    #[test]
    fn test_strip_markdown_code_block_with_newlines() {
        // Test with extra newlines around the JSON (like the actual LLM response)
        let wrapped = r#"```json
{
  "title": "Test"
}
```"#;

        let stripped = strip_markdown_code_block(wrapped);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();

        assert_eq!(parsed["title"], "Test");
    }

    #[test]
    fn test_strip_markdown_code_block_no_closing_fence() {
        // Some LLMs might forget the closing fence
        let wrapped = r#"```json
{
  "title": "Test"
}"#;

        let stripped = strip_markdown_code_block(wrapped);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();

        assert_eq!(parsed["title"], "Test");
    }

    #[test]
    fn test_strip_leading_think_blocks_removes_reasoning_preamble() {
        let wrapped = r#"<think>Reasoning goes here</think>

{"title":"Test"}"#;

        let stripped = strip_leading_think_blocks(wrapped);

        assert_eq!(stripped, r#"{"title":"Test"}"#);
    }

    #[test]
    fn test_extract_first_json_object_ignores_prefix_and_suffix_text() {
        let content = r#"Some prose before
{"title":"Test","description":"Example","is_non_trivial":false,"subtasks":[]}
Some prose after"#;

        let extracted = extract_first_json_object(content).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&extracted).unwrap();

        assert_eq!(parsed["title"], "Test");
        assert_eq!(parsed["is_non_trivial"], false);
    }

    #[test]
    fn test_normalize_planning_json_handles_think_and_fenced_json() {
        let wrapped = r#"<think>The user said hi, so this is trivial.</think>

```json
{
  "title": "Greeting Response",
  "description": "Acknowledge the user's greeting",
  "is_non_trivial": false,
  "subtasks": []
}
```"#;

        let normalized = normalize_planning_json(wrapped).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&normalized).unwrap();

        assert_eq!(parsed["title"], "Greeting Response");
        assert_eq!(parsed["is_non_trivial"], false);
    }

    #[test]
    fn test_normalize_planning_json_rejects_missing_json() {
        // Prose that survives `<think>` stripping but contains no JSON object —
        // exercises the "Could not locate a JSON object" branch of
        // `normalize_planning_json`. An input like `<think>...</think>` alone
        // would strip to empty and hit the empty-after-normalization branch
        // instead, which is covered elsewhere.
        let error =
            normalize_planning_json("I'm sorry, I can't help with that request.").unwrap_err();

        assert!(
            error.contains("Could not locate a JSON object"),
            "expected missing-JSON error, got: {error}"
        );
    }

    // ========================================================================
    // Behavior Tests with Mock LLM
    // ========================================================================

    /// Helper function to test planning with a mock client
    async fn plan_new_task_with_mock(
        client: &MockLlmClient,
        user_message: &str,
    ) -> Result<TaskPlan, String> {
        let system_prompt = build_planning_system_prompt();
        let user_prompt = format!(
            "User Request: {}\n\nCreate a task plan for this request.",
            user_message
        );

        let request = ChatRequest::new(
            "test-model",
            vec![
                ChatMessage {
                    role: MessageRole::System,
                    content: MessageContent::Text(system_prompt),
                    tool_calls: Vec::new(),
                    tool_result: None,
                },
                ChatMessage {
                    role: MessageRole::User,
                    content: MessageContent::Text(user_prompt),
                    tool_calls: Vec::new(),
                    tool_result: None,
                },
            ],
        );

        let response = client
            .chat(request)
            .await
            .map_err(|e| format!("LLM error: {e}"))?;

        let content = normalize_planning_json(&response.content)?;

        let plan: TaskPlan = serde_json::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse planning response as JSON: {}\n\nResponse was: {}",
                e, content
            )
        })?;

        Ok(plan)
    }

    /// Helper function to test replanning with a mock client
    async fn review_and_replan_with_mock(
        client: &MockLlmClient,
        current_plan: &TaskPlan,
        progress_summary: &str,
        user_message: &str,
    ) -> Result<ReplanOutput, String> {
        let system_prompt = build_replan_system_prompt();
        let user_prompt = format!(
            "Current Plan:\n{}\n\nProgress Summary:\n{}\n\nUser's New Request:\n{}\n\nAnalyze and decide what to do next.",
            serde_json::to_string(current_plan).unwrap_or_default(),
            progress_summary,
            user_message
        );

        let request = ChatRequest::new(
            "test-model",
            vec![
                ChatMessage {
                    role: MessageRole::System,
                    content: MessageContent::Text(system_prompt),
                    tool_calls: Vec::new(),
                    tool_result: None,
                },
                ChatMessage {
                    role: MessageRole::User,
                    content: MessageContent::Text(user_prompt),
                    tool_calls: Vec::new(),
                    tool_result: None,
                },
            ],
        );

        let response = client
            .chat(request)
            .await
            .map_err(|e| format!("LLM error: {e}"))?;

        let content = normalize_planning_json(&response.content)?;

        let output: ReplanOutput = serde_json::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse replan response as JSON: {}\n\nResponse was: {}",
                e, content
            )
        })?;

        Ok(output)
    }

    #[tokio::test]
    async fn test_plan_new_task_with_simple_request() {
        let client = MockLlmClient::anthropic("test-model");
        client.set_response(
            serde_json::json!({
                "title": "Read README file",
                "description": "Read the README.md file",
                "is_non_trivial": false,
                "subtasks": []
            })
            .to_string(),
        );

        let plan = plan_new_task_with_mock(&client, "Read the README file")
            .await
            .unwrap();

        assert_eq!(plan.title, "Read README file");
        assert_eq!(plan.description, "Read the README.md file");
        assert!(!plan.is_non_trivial);
        assert!(plan.subtasks.is_empty());
    }

    #[tokio::test]
    async fn test_plan_new_task_with_complex_request() {
        let client = MockLlmClient::anthropic("test-model");
        client.set_response(serde_json::json!({
            "title": "Codebase investigation and analysis",
            "description": "Investigate the codebase and identify improvement opportunities",
            "is_non_trivial": true,
            "subtasks": [
                {"title": "Scan project structure", "description": "List all source files", "order": 0},
                {"title": "Analyze dependencies", "description": "Review external dependencies", "order": 1},
                {"title": "Identify improvements", "description": "List potential improvements", "order": 2}
            ]
        }).to_string());

        let plan =
            plan_new_task_with_mock(&client, "Investigate codebase and suggest improvements")
                .await
                .unwrap();

        assert!(plan.title.len() <= 80);
        assert!(plan.is_non_trivial);
        assert_eq!(plan.subtasks.len(), 3);
        assert_eq!(plan.subtasks[0].order, 0);
        assert_eq!(plan.subtasks[1].order, 1);
        assert_eq!(plan.subtasks[2].order, 2);
    }

    #[tokio::test]
    async fn test_plan_new_task_handles_reasoning_wrapped_json() {
        let client = MockLlmClient::anthropic("test-model");
        client.set_response(
            r#"<think>The user just said hi, so this task is trivial.</think>

```json
{
  "title": "Respond to Greeting",
  "description": "Acknowledge the user's simple greeting with a friendly response",
  "is_non_trivial": false,
  "subtasks": []
}
```"#
                .to_string(),
        );

        let plan = plan_new_task_with_mock(&client, "Hi").await.unwrap();

        assert_eq!(plan.title, "Respond to Greeting");
        assert!(!plan.is_non_trivial);
        assert!(plan.subtasks.is_empty());
    }

    #[tokio::test]
    async fn test_plan_new_task_handles_invalid_json() {
        let client = MockLlmClient::anthropic("test-model");
        client.set_response("This is not valid JSON".to_string());

        let result = plan_new_task_with_mock(&client, "Some request").await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        // `normalize_planning_json` rejects responses with no JSON object before
        // `serde_json::from_str` ever runs, so either error-path is an acceptable
        // failure mode for "not valid JSON".
        assert!(
            error.contains("Failed to parse") || error.contains("Could not locate a JSON object"),
            "expected parse or locate error, got: {error}"
        );
    }

    #[tokio::test]
    async fn test_review_and_replan_abort_decision() {
        let client = MockLlmClient::anthropic("test-model");
        client.set_response(
            serde_json::json!({
                "decision": "abort",
                "completed_indices": [0],
                "updated_plan": null,
                "user_message": "Switching to cooking topic",
                "next_step_index": null
            })
            .to_string(),
        );

        let current_task = create_test_task("Investigate Rust code");
        let subtasks = vec![create_test_subtask("Scan files", 0)];

        let current_plan = build_plan_from_task(&current_task, &subtasks);

        let output = review_and_replan_with_mock(
            &client,
            &current_plan,
            "Completed 1/3 subtasks",
            "How do I cook pasta?",
        )
        .await
        .unwrap();

        assert_eq!(output.decision, ReplanAction::Abort);
        assert!(output.updated_plan.is_none());
        assert!(output.next_step_index.is_none());
    }

    #[tokio::test]
    async fn test_review_and_replan_continue_decision() {
        let client = MockLlmClient::anthropic("test-model");
        client.set_response(
            serde_json::json!({
                "decision": "continue",
                "completed_indices": [],
                "updated_plan": null,
                "user_message": "Continuing with clarification",
                "next_step_index": null
            })
            .to_string(),
        );

        let current_task = create_test_task("Fix bug in auth");
        let current_plan = build_plan_from_task(&current_task, &[]);

        let output = review_and_replan_with_mock(
            &client,
            &current_plan,
            "In progress on subtask 1",
            "Yes, proceed with the fix",
        )
        .await
        .unwrap();

        assert_eq!(output.decision, ReplanAction::Continue);
    }

    #[tokio::test]
    async fn test_review_and_replan_updates_plan() {
        let client = MockLlmClient::anthropic("test-model");
        client.set_response(serde_json::json!({
            "decision": "replan",
            "completed_indices": [0, 1],
            "updated_plan": {
                "title": "Codebase investigation with performance focus",
                "description": "Investigate and optimize performance",
                "is_non_trivial": true,
                "subtasks": [
                    {"title": "Profile database queries", "description": "Find slow queries", "order": 0}
                ]
            },
            "user_message": "Adding performance optimization to investigation",
            "next_step_index": 0
        }).to_string());

        let current_task = create_test_task("Investigate codebase");
        let subtasks = vec![
            create_test_subtask("Scan files", 0),
            create_test_subtask("Analyze structure", 1),
        ];
        let current_plan = build_plan_from_task(&current_task, &subtasks);

        let output = review_and_replan_with_mock(
            &client,
            &current_plan,
            "Completed scanning and analysis",
            "Also focus on performance",
        )
        .await
        .unwrap();

        assert_eq!(output.decision, ReplanAction::Replan);
        assert!(output.updated_plan.is_some());
        assert_eq!(output.completed_indices, vec![0, 1]);
        assert_eq!(output.next_step_index, Some(0));
        assert!(output.user_message.contains("performance"));
    }

    #[tokio::test]
    async fn test_review_and_replan_handles_reasoning_wrapped_json() {
        let client = MockLlmClient::anthropic("test-model");
        client.set_response(
            r#"<think>The request is related, so replanning is appropriate.</think>

{
  "decision": "replan",
  "completed_indices": [0],
  "updated_plan": {
    "title": "Investigate planner parsing",
    "description": "Harden planner parsing against reasoning wrappers",
    "is_non_trivial": true,
    "subtasks": [
      {
        "title": "Add parser normalization",
        "description": "Strip wrapper content before JSON parsing",
        "order": 0
      }
    ]
  },
  "user_message": "Replanning around the parsing failure",
  "next_step_index": 0
}"#
            .to_string(),
        );

        let current_task = create_test_task("Investigate planner");
        let current_plan = build_plan_from_task(&current_task, &[]);

        let output = review_and_replan_with_mock(
            &client,
            &current_plan,
            "Captured the failing provider response",
            "Focus on parsing wrappers first",
        )
        .await
        .unwrap();

        assert_eq!(output.decision, ReplanAction::Replan);
        assert_eq!(output.completed_indices, vec![0]);
        assert_eq!(output.next_step_index, Some(0));
        assert_eq!(
            output.updated_plan.as_ref().map(|plan| plan.title.as_str()),
            Some("Investigate planner parsing")
        );
    }

    // ========================================================================
    // Test Helper Functions
    // ========================================================================

    fn create_test_task(title: &str) -> AgentTask {
        AgentTask {
            id: "task-1".to_string(),
            session_id: "session-1".to_string(),
            parent_task_id: None,
            title: title.to_string(),
            description: Some("Test task".to_string()),
            status: TaskStatus::InProgress,
            order: 0,
            mode: Some(TaskMode::FullAccess),
            needs_review: false,
        }
    }

    fn create_test_subtask(title: &str, order: i32) -> AgentTask {
        AgentTask {
            id: format!("subtask-{}", order),
            session_id: "session-1".to_string(),
            parent_task_id: Some("task-1".to_string()),
            title: title.to_string(),
            description: None,
            status: TaskStatus::Completed,
            order,
            mode: Some(TaskMode::FullAccess),
            needs_review: false,
        }
    }

    fn build_plan_from_task(task: &AgentTask, subtasks: &[AgentTask]) -> TaskPlan {
        TaskPlan {
            title: task.title.clone(),
            description: task.description.clone().unwrap_or_default(),
            is_non_trivial: !subtasks.is_empty(),
            subtasks: subtasks
                .iter()
                .map(|st| SubtaskPlan {
                    title: st.title.clone(),
                    description: st.description.clone(),
                    order: st.order,
                })
                .collect(),
        }
    }
}
