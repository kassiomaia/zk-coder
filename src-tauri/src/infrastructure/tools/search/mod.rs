//! Web search tools.

mod google_scrape;
mod grep;
pub(crate) mod web_search;

use crate::infrastructure::tools::traits::{Tool, ToolCategory};
pub use google_scrape::GoogleScrapeTool;
pub use grep::GrepTool;
pub use web_search::WebSearchTool;

/// Search tools category for web searching and scraping.
pub struct SearchCategory;

impl ToolCategory for SearchCategory {
    fn name(&self) -> &'static str {
        "search"
    }

    fn description(&self) -> &'static str {
        "Workspace and web search tools"
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![
            Box::new(GrepTool),
            Box::new(GoogleScrapeTool),
            Box::new(WebSearchTool::new_production()),
        ]
    }
}
