//! PPTX tool category registration.

use super::{PptxAddSlideTool, PptxDeleteSlideTool, PptxFinalizeTool, PptxGetSlideTool, PptxListSlidesTool, PptxSearchSlidesTool, PptxStartTool, PptxUpdateSlideTool};
use crate::infrastructure::tools::traits::ToolCategory;

/// PPTX document tools category.
pub struct PptxCategory;

impl ToolCategory for PptxCategory {
    fn name(&self) -> &'static str {
        "pptx"
    }

    fn description(&self) -> &'static str {
        "Create and manipulate PowerPoint presentations"
    }

    fn tools(&self) -> Vec<Box<dyn crate::infrastructure::tools::traits::Tool>> {
        vec![
            Box::new(PptxStartTool),
            Box::new(PptxAddSlideTool),
            Box::new(PptxListSlidesTool),
            Box::new(PptxGetSlideTool),
            Box::new(PptxSearchSlidesTool),
            Box::new(PptxUpdateSlideTool),
            Box::new(PptxDeleteSlideTool),
            Box::new(PptxFinalizeTool),
        ]
    }
}
