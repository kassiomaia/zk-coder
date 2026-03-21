//! System/app information tools.

mod get_app_info;

pub use get_app_info::GetAppInfoTool;
use crate::tools::{Tool, ToolCategory};

/// System tools category for application metadata.
pub struct SystemCategory;

impl ToolCategory for SystemCategory {
    fn name(&self) -> &'static str {
        "system"
    }

    fn description(&self) -> &'static str {
        "Application and system information tools"
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![Box::new(GetAppInfoTool)]
    }
}
