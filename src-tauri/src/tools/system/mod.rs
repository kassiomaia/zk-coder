//! System/app information and command execution tools.

mod execute_command;
mod get_app_info;

pub use execute_command::ExecuteCommandTool;
pub use get_app_info::GetAppInfoTool;
use crate::tools::{Tool, ToolCategory};

/// System tools category for application metadata and command execution.
pub struct SystemCategory;

impl ToolCategory for SystemCategory {
    fn name(&self) -> &'static str {
        "system"
    }

    fn description(&self) -> &'static str {
        "Application information and command execution tools"
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![
            Box::new(GetAppInfoTool),
            Box::new(ExecuteCommandTool),
        ]
    }
}
