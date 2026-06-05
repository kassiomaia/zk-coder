//! Single-shot planner — produces a fresh `TaskPlan` for a brand-new task.

use crate::core::config::ConfigRepository;
use crate::core::llm::entities::{ChatMessage, ChatRequest, MessageContent, MessageRole};
use crate::infrastructure::agent::runtime::client::create_llm_client;
use log::info;

use super::parsing::{finish_reason_indicates_truncation, normalize_planning_json};
use super::{TaskPlan, PLANNER_MAX_OUTPUT_TOKENS};

/// Create a task plan using single-shot LLM planning.
///
/// This generates a title, description, and optionally subtasks all in one LLM call.
pub async fn plan_new_task(
    config_repository: &dyn ConfigRepository,
    user_message: &str,
    model_id: Option<&str>,
) -> Result<TaskPlan, String> {
    let (client, actual_model) = create_llm_client(
        config_repository,
        model_id.map(std::string::ToString::to_string),
    )
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
    .with_max_tokens(PLANNER_MAX_OUTPUT_TOKENS)
    .with_temperature(0.3);

    // Execute LLM call
    let response = client
        .chat(request)
        .await
        .map_err(|e| format!("LLM planning failed: {e}"))?;

    // Log raw response for debugging
    info!("[Planner] Raw LLM response: {}", &response.content);

    // Detect truncation before JSON extraction so the user sees a
    // targeted error instead of the cryptic "Could not locate a JSON
    // object" that falls out of parsing a half-written object.
    if finish_reason_indicates_truncation(response.finish_reason.as_deref()) {
        return Err(format!(
            "Planner output hit the {PLANNER_MAX_OUTPUT_TOKENS}-token cap before finishing the \
             JSON plan. Ask for a narrower task or raise the planner token budget. \
             Partial response: '{}'",
            response.content
        ));
    }

    let content = normalize_planning_json(&response.content)?;

    // Log stripped content for debugging
    info!("[Planner] Stripped content: {}", &content);

    // Parse JSON response
    let plan: TaskPlan = serde_json::from_str(&content).map_err(|e| {
        format!("Failed to parse planning response as JSON: {e}\n\nResponse was: {content}")
    })?;

    Ok(plan)
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

#[cfg(test)]
mod tests {
    use super::super::parsing::normalize_planning_json;
    use super::super::TaskPlan;
    use super::*;
    use crate::infrastructure::llm::mock::MockLlmClient;
    use crate::infrastructure::llm::LlmClient;

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
}
