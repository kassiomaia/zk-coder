//! Filesystem manipulation tools.

mod list_files;
mod read_file;
pub mod write_file;

use crate::infrastructure::tools::traits::{Tool, ToolCategory};
pub use list_files::ListFilesTool;
pub use read_file::ReadFileTool;
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
            Box::new(WriteFileTool),
        ]
    }
}
