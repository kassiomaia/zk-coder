//! Filesystem manipulation tools.

mod list_files;
mod read_file;

pub use list_files::ListFilesTool;
pub use read_file::ReadFileTool;
use crate::tools::traits::{Tool, ToolCategory};

/// Filesystem tools category for file operations.
pub struct FsCategory;

impl ToolCategory for FsCategory {
    fn name(&self) -> &'static str {
        "fs"
    }

    fn description(&self) -> &'static str {
        "Filesystem tools for reading, listing, and exploring workspace files"
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![Box::new(ReadFileTool), Box::new(ListFilesTool)]
    }
}
