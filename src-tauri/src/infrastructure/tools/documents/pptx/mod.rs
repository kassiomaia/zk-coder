//! PPTX (`PowerPoint`) document tools.

mod add_slide;
mod category;
mod delete_slide;
mod r#final;
mod list;
mod r#start;
mod update_slide;
mod validate;

pub use add_slide::PptxAddSlideTool;
pub use category::PptxCategory;
pub use delete_slide::PptxDeleteSlideTool;
pub use list::{PptxGetSlideTool, PptxListSlidesTool, PptxSearchSlidesTool};
pub use r#final::PptxFinalizeTool;
pub use r#start::PptxStartTool;
pub use update_slide::PptxUpdateSlideTool;
pub use validate::PptxValidator;
