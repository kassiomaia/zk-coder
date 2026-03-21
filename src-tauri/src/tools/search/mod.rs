//! Web search tools.

mod google_scrape;

pub use google_scrape::GoogleScrapeTool;
use crate::tools::traits::{Tool, ToolCategory};

/// Search tools category for web searching and scraping.
pub struct SearchCategory;

impl ToolCategory for SearchCategory {
    fn name(&self) -> &'static str {
        "search"
    }

    fn description(&self) -> &'static str {
        "Web search and scraping tools"
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![Box::new(GoogleScrapeTool)]
    }
}
