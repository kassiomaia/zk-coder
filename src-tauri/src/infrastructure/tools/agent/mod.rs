//! Agent communication tools.
//!
//! Tools that allow agents to communicate status and progress to users.

mod advance_to_next_subtask;
mod emit_status;
mod mark_subtask_status;
mod narrate;
mod plan_subtasks;

use crate::infrastructure::tools::{Tool, ToolCategory};
pub use advance_to_next_subtask::AdvanceToNextSubtaskTool;
pub use emit_status::EmitStatusTool;
pub use mark_subtask_status::MarkSubtaskStatusTool;
pub use narrate::NarrateTool;
pub use plan_subtasks::PlanSubtasksTool;

/// Agent tools category for status communication.
pub struct AgentCategory;

impl ToolCategory for AgentCategory {
    fn name(&self) -> &'static str {
        "agent"
    }

    fn description(&self) -> &'static str {
        "Agent communication and status tools"
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![
            Box::new(EmitStatusTool),
            Box::new(NarrateTool),
            Box::new(PlanSubtasksTool),
            Box::new(AdvanceToNextSubtaskTool),
            Box::new(MarkSubtaskStatusTool),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_category_has_five_tools() {
        let category = AgentCategory;
        let tools = category.tools();

        assert_eq!(tools.len(), 5, "Should have 5 agent tools");

        let tool_names: Vec<String> = tools.iter().map(|t| t.meta().id.to_string()).collect();

        assert!(tool_names.contains(&"agent_emit_status".to_string()));
        assert!(tool_names.contains(&"agent_narrate".to_string()));
        assert!(tool_names.contains(&"agent_plan_subtasks".to_string()));
        assert!(tool_names.contains(&"agent_advance_to_next_subtask".to_string()));
        assert!(tool_names.contains(&"agent_mark_subtask_status".to_string()));
    }

    #[test]
    fn test_tool_ids_match_allowed_tools() {
        let category = AgentCategory;
        let tools = category.tools();

        let tool_names: Vec<String> = tools
            .iter()
            .map(|t| {
                let id = t.meta().id.to_string();
                // Convert "agent.advance_to_next_subtask" to "agent_advance_to_next_subtask"
                id.replace('.', "_")
            })
            .collect();

        // These should match the allowed_tools in registry.rs
        assert!(tool_names.contains(&"agent_emit_status".to_string()));
        assert!(tool_names.contains(&"agent_narrate".to_string()));
        assert!(tool_names.contains(&"agent_plan_subtasks".to_string()));
        assert!(tool_names.contains(&"agent_advance_to_next_subtask".to_string()));
        assert!(tool_names.contains(&"agent_mark_subtask_status".to_string()));
    }

    #[test]
    fn test_agent_category_name() {
        let category = AgentCategory;
        assert_eq!(category.name(), "agent");
    }

    #[test]
    fn test_agent_category_description() {
        let category = AgentCategory;
        assert!(!category.description().is_empty());
    }

    #[test]
    fn test_tool_descriptions() {
        let category = AgentCategory;
        let tools = category.tools();

        for tool in tools {
            let meta = tool.meta();
            assert!(
                !meta.description.is_empty(),
                "Tool {} should have a description",
                meta.id
            );
        }
    }

    #[test]
    fn test_all_agent_tools_dont_require_workspace() {
        let category = AgentCategory;
        let tools = category.tools();

        for tool in tools {
            let meta = tool.meta();
            assert!(
                !meta.requires_workspace,
                "Tool {} should not require workspace",
                meta.id
            );
        }
    }

    #[test]
    fn test_all_agent_tools_are_not_experimental() {
        let category = AgentCategory;
        let tools = category.tools();

        for tool in tools {
            let meta = tool.meta();
            assert!(
                !meta.experimental,
                "Tool {} should not be experimental",
                meta.id
            );
        }
    }
}
