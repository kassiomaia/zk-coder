//! PDF tool category registration.

use super::{
    PdfAddPageTool, PdfDeletePageTool, PdfFinalizeTool, PdfGetPageTool, PdfListPagesTool,
    PdfStartTool,
};
use crate::infrastructure::tools::traits::ToolCategory;

/// PDF document tools category.
pub struct PdfCategory;

impl ToolCategory for PdfCategory {
    fn name(&self) -> &'static str {
        "pdf"
    }

    fn description(&self) -> &'static str {
        "Create and manipulate PDF documents"
    }

    fn tools(&self) -> Vec<Box<dyn crate::infrastructure::tools::traits::Tool>> {
        vec![
            Box::new(PdfStartTool),
            Box::new(PdfAddPageTool),
            Box::new(PdfListPagesTool),
            Box::new(PdfGetPageTool),
            Box::new(PdfDeletePageTool),
            Box::new(PdfFinalizeTool),
        ]
    }
}
