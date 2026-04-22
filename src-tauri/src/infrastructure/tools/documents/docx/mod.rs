//! DOCX (Word document) tools.

mod add_section;
mod r#final;
mod generator;
mod r#list;
mod schema;
mod start;
mod validate;

pub use add_section::DocxAddSectionTool;
pub use r#final::DocxFinalizeTool;
pub use r#list::DocxListSectionsTool;
pub use start::DocxStartTool;
pub use validate::DocxValidator;

use crate::infrastructure::tools::traits::{Tool, ToolCategory};

/// DOCX tools category.
pub struct DocxCategory;

impl ToolCategory for DocxCategory {
    fn name(&self) -> &'static str {
        "docx"
    }

    fn description(&self) -> &'static str {
        "Word document tools for incremental building, editing, and finalizing DOCX files"
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![
            Box::new(DocxStartTool),
            Box::new(DocxAddSectionTool),
            Box::new(DocxListSectionsTool),
            Box::new(DocxFinalizeTool),
        ]
    }
}
