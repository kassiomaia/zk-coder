//! Replanner — reviews an interrupted task and decides whether to
//! continue, replan, or abort.

use crate::core::agent::task::SubTask;
use crate::core::config::ConfigRepository;
use crate::core::llm::entities::{ChatMessage, ChatRequest, MessageContent, MessageRole};
use crate::infrastructure::agent::runtime::client::create_llm_client;
use serde::{Deserialize, Serialize};

use super::parsing::normalize_planning_json;
use super::{SubtaskPlan, TaskPlan};

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

/// Review an interrupted task and decide on next action.
///
/// This is called when a user sends a message while a task is incomplete.
pub async fn review_and_replan(
    config_repository: &dyn ConfigRepository,
    current_task: &crate::core::agent::task::AgentTask,
    subtasks: &[SubTask],
    user_message: &str,
    progress_summary: &str,
    model_id: Option<&str>,
) -> Result<ReplanOutput, String> {
    let (client, actual_model) =
        create_llm_client(config_repository, model_id.map(std::string::ToString::to_string))
            .await
            .map_err(|e| format!("Failed to create LLM client for replanning: {e}"))?;

    // Build current plan summary
    let current_plan = TaskPlan {
        title: current_task.title.clone().unwrap_or_default(),
        description: current_task
            .request
            .context
            .as_ref()
            .and_then(|ctx| ctx.selection.clone())
            .unwrap_or_default(),
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

#[cfg(test)]
mod tests {
    use super::super::parsing::normalize_planning_json;
    use super::super::test_support::{build_plan_from_task, create_test_subtask, create_test_task};
    use super::super::TaskPlan;
    use super::*;
    use crate::infrastructure::llm::mock::MockLlmClient;
    use crate::infrastructure::llm::LlmClient;

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
}
