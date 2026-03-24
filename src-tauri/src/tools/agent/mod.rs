//! Agent communication tools.
//!
//! Tools that allow agents to communicate status and progress to users.

mod emit_status;
mod plan_subtasks;

pub use emit_status::EmitStatusTool;
pub use plan_subtasks::PlanSubtasksTool;
use crate::tools::{Tool, ToolCategory};

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
		vec![Box::new(EmitStatusTool), Box::new(PlanSubtasksTool)]
	}
}
