//! Agent planning phase.
//!
//! Before execution begins, the planner analyzes the user's request
//! and generates a structured execution plan.

use log::{error, info};
use serde::{Deserialize, Serialize};

use crate::llm::{ChatRequest, LlmClient, Message};

use super::task::{Complexity, Plan, PlanStep};

/// Planner errors.
#[derive(Debug, thiserror::Error)]
pub enum PlannerError {
    #[error("LLM error: {0}")]
    Llm(String),
    #[error("Failed to parse plan: {0}")]
    Parse(String),
}

/// Result of a planning attempt.
pub type PlannerResult<T> = Result<T, PlannerError>;

/// The agent planner generates execution plans.
pub struct Planner {
    model_name: String,
    system_prompt: String,
}

impl Planner {
    /// Create a new planner with default configuration.
    pub fn new(model_name: String) -> Self {
        Self {
            model_name,
            system_prompt: Self::default_system_prompt(),
        }
    }

    /// Create a planner with a custom system prompt.
    pub fn with_prompt(mut self, prompt: String) -> Self {
        self.system_prompt = prompt;
        self
    }

    /// Generate an execution plan from the user's request.
    pub async fn generate_plan(
        &self,
        client: &dyn LlmClient,
        user_request: &str,
        workspace_context: Option<&str>,
    ) -> PlannerResult<Plan> {
        info!("[Planner] Generating plan for request: {}", user_request);

        // Build messages
        let mut messages = vec![Message::system(&self.system_prompt)];

        // Add workspace context if available
        if let Some(ctx) = workspace_context {
            messages.push(Message::user(format!(
                "Workspace Context:\n{}\n\nUser Request: {}",
                ctx, user_request
            )));
        } else {
            messages.push(Message::user(format!("User Request: {}", user_request)));
        }

        // Call LLM
        let request = ChatRequest {
            model: self.model_name.clone(),
            messages,
            temperature: Some(0.3), // Lower temperature for more structured output
            max_tokens: Some(2000),
            tools: vec![],
            system: None,
            top_p: None,
            stop: None,
            stream: false,
        };

        let response = client.chat(request).await.map_err(|e| {
            error!("[Planner] LLM call failed: {}", e);
            PlannerError::Llm(e.to_string())
        })?;

        // ChatResponse has content: String directly
        let content = if response.content.trim().is_empty() {
            return Err(PlannerError::Parse("Empty response from LLM".to_string()));
        } else {
            response.content
        };

        // Parse the plan from JSON
        let plan_response: PlanResponse = serde_json::from_str(&content).map_err(|e| {
            error!("[Planner] Failed to parse plan JSON: {}", e);
            PlannerError::Parse(format!("Invalid JSON: {}", e))
        })?;

        let plan = plan_response.to_plan();
        info!(
            "[Planner] Generated plan with {} steps: {}",
            plan.steps.len(),
            plan.summary
        );

        Ok(plan)
    }

    /// Default system prompt for the planner.
    fn default_system_prompt() -> String {
        r#"You are a planning assistant. Your job is to analyze the user's request and break it down into a structured execution plan.

## Your Task

1. Understand what the user wants to accomplish
2. Break it down into 3-8 logical, executable steps
3. Order steps by dependency (what must be done first)
4. Estimate complexity for each step

## Output Format

Return ONLY a JSON object in this exact format:

{
  "summary": "Brief one-sentence description of what will be done",
  "steps": [
    {
      "title": "Step title (verb-based, specific)",
      "description": "Detailed explanation of what this step accomplishes",
      "complexity": "simple" | "medium" | "complex"
    }
  ]
}

## Guidelines

- **Titles**: Use action verbs (e.g., "Analyze codebase", not "Codebase analysis")
- **Complexity**:
  - "simple": Single file, straightforward change (< 15 min)
  - "medium": Multiple files or moderate complexity (15-60 min)
  - "complex": Many files, architectural changes (> 1 hour)
- **Ordering**: List steps in execution order
- **Count**: 3-8 steps is ideal. More than 8 means you should group related steps.

## Examples

User: "Add authentication to the app"
Response:
{
  "summary": "Implement user authentication with JWT",
  "steps": [
    {
      "title": "Analyze existing codebase structure",
      "description": "Review the current app structure to understand where authentication should be integrated",
      "complexity": "simple"
    },
    {
      "title": "Design authentication schema",
      "description": "Define user model, token storage approach, and authentication flow",
      "complexity": "medium"
    },
    {
      "title": "Create authentication module",
      "description": "Implement login, logout, and token validation logic",
      "complexity": "complex"
    },
    {
      "title": "Add authentication middleware",
      "description": "Protect routes that require authentication",
      "complexity": "medium"
    },
    {
      "title": "Create login UI components",
      "description": "Build login form and integrate with authentication module",
      "complexity": "medium"
    }
  ]
}

Remember: Return ONLY valid JSON. No markdown, no explanation, just the JSON."#.to_string()
    }
}

/// JSON response format from the planner LLM.
#[derive(Debug, Serialize, Deserialize)]
struct PlanResponse {
    summary: String,
    steps: Vec<PlanStepResponse>,
}

/// Single step in the planner response.
#[derive(Debug, Serialize, Deserialize)]
struct PlanStepResponse {
    title: String,
    description: Option<String>,
    complexity: String,
}

impl PlanResponse {
    /// Convert the response to a Plan.
    fn to_plan(self) -> Plan {
        let steps = self
            .steps
            .into_iter()
            .map(|step| {
                let complexity = match step.complexity.to_lowercase().as_str() {
                    "simple" => Complexity::Simple,
                    "medium" => Complexity::Medium,
                    "complex" => Complexity::Complex,
                    _ => Complexity::Medium,
                };
                PlanStep {
                    title: step.title,
                    description: step.description,
                    complexity,
                }
            })
            .collect();

        Plan::new(self.summary, steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_step_new() {
        let step = PlanStep::new("Do something".to_string());
        assert_eq!(step.title, "Do something");
        assert_eq!(step.complexity, Complexity::Medium);
        assert!(step.description.is_none());
    }

    #[test]
    fn test_plan_step_with_description() {
        let step = PlanStep::new("Do something".to_string())
            .with_description("Details".to_string())
            .with_complexity(Complexity::Simple);
        assert_eq!(step.description, Some("Details".to_string()));
        assert_eq!(step.complexity, Complexity::Simple);
    }

    #[test]
    fn test_plan_to_subtasks() {
        let plan = Plan::new(
            "Test plan".to_string(),
            vec![
                PlanStep::new("Step 1".to_string()),
                PlanStep::new("Step 2".to_string()),
            ],
        );

        let subtasks = plan.to_subtasks("task-123");
        assert_eq!(subtasks.len(), 2);
        assert_eq!(subtasks[0].title, "Step 1");
        assert_eq!(subtasks[0].task_id, "task-123");
        assert_eq!(subtasks[0].order, 0);
        assert_eq!(subtasks[1].order, 1);
    }

    #[test]
    fn test_plan_response_to_plan() {
        let json = r#"{
            "summary": "Test summary",
            "steps": [
                {
                    "title": "Step 1",
                    "description": "First step",
                    "complexity": "simple"
                }
            ]
        }"#;

        let response: PlanResponse = serde_json::from_str(json).unwrap();
        let plan = response.to_plan();

        assert_eq!(plan.summary, "Test summary");
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].title, "Step 1");
        assert_eq!(plan.steps[0].complexity, Complexity::Simple);
    }
}
