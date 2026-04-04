//! Filesystem manipulation tools.

mod apply_patch;
mod fuzzy_find_files;
mod glob_files;
mod list_files;
mod read_file;
mod replace_in_file;
mod stat_path;
pub mod write_file;

use crate::infrastructure::tools::traits::{Tool, ToolCategory};
pub use apply_patch::{generate_preview_diff as generate_apply_patch_preview_diff, ApplyPatchTool};
pub use fuzzy_find_files::FuzzyFindFilesTool;
pub use glob_files::GlobFilesTool;
pub use list_files::ListFilesTool;
pub use read_file::ReadFileTool;
pub use replace_in_file::{generate_preview_diff as generate_replace_preview_diff, ReplaceInFileTool};
pub use stat_path::StatPathTool;
pub use write_file::WriteFileTool;

/// Filesystem tools category for file operations.
pub struct FsCategory;

impl ToolCategory for FsCategory {
    fn name(&self) -> &'static str {
        "fs"
    }

    fn description(&self) -> &'static str {
        "Filesystem tools for reading, listing, writing, and exploring workspace files"
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![
            Box::new(ReadFileTool),
            Box::new(ListFilesTool),
            Box::new(GlobFilesTool),
            Box::new(StatPathTool),
            Box::new(FuzzyFindFilesTool),
            Box::new(ReplaceInFileTool),
            Box::new(ApplyPatchTool),
            Box::new(WriteFileTool),
        ]
    }
}
