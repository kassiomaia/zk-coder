//! List, get, and search slides in a PPTX presentation.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::{get_draft_store, DraftStoreExt},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use tauri::AppHandle;

// ============================================================================
// List Slides
// ============================================================================

/// Arguments for listing slides.
#[derive(Debug, Deserialize)]
pub struct PptxListSlidesArgs {
    /// Deck ID from pptx_start
    pub deck_id: String,
    /// Whether to include full content (default: false)
    #[serde(default)]
    pub include_content: bool,
}

/// Output from listing slides.
#[derive(Debug, Serialize)]
pub struct PptxListSlidesOutput {
    /// Deck ID
    pub deck_id: String,
    /// List of slides
    pub slides: Vec<SlideSummary>,
    /// Total slide count
    pub total_slides: usize,
}

/// Summary of a slide.
#[derive(Debug, Serialize)]
pub struct SlideSummary {
    /// Slide ID
    pub slide_id: String,
    /// Slide number (1-based)
    pub slide_number: usize,
    /// Layout type
    pub layout: String,
    /// Slide title
    pub title: String,
    /// Content preview (first 100 chars)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_preview: Option<String>,
    /// Whether slide has notes
    pub has_notes: bool,
}

/// Tool to list slides in a PPTX presentation.
pub struct PptxListSlidesTool;

impl Tool for PptxListSlidesTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("pptx", "list_slides"),
            description: "List all slides in a PowerPoint presentation, showing titles and layouts. Use this to understand the presentation structure before making modifications.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "deck_id": {
                        "type": "string",
                        "description": "Deck ID from pptx_start"
                    },
                    "include_content": {
                        "type": "boolean",
                        "description": "Whether to include full slide content (default: false)",
                        "default": false
                    }
                },
                "required": ["deck_id"]
            }),
            requires_workspace: false,
            experimental: false,
            feature_flag: None,
        }
    }

    fn execute(
        &self,
        _context: ToolExecutionContext,
        arguments: JsonValue,
        _app_handle: AppHandle,
        _workspace_root: Option<PathBuf>,
    ) -> ToolFuture {
        Box::pin(async move {
            let args: PptxListSlidesArgs = parse_args("pptx_list_slides", arguments)?;

            let draft_store = get_draft_store();
            let slides: Vec<SlideSummary> = {
                let store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {}", e))
                })?;

                let deck = store
                    .get_pptx_draft(&args.deck_id)
                    .ok_or_else(|| ToolError::failed(format!("Deck not found: {}", args.deck_id)))?;

                deck.slides
                    .iter()
                    .map(|draft_slide| SlideSummary {
                        slide_id: draft_slide.id.clone(),
                        slide_number: draft_slide.index + 1,
                        layout: format!("{:?}", draft_slide.slide.layout),
                        title: draft_slide.slide.title.clone(),
                        content_preview: if args.include_content {
                            Some(format_content_preview(&draft_slide.slide.content))
                        } else {
                            None
                        },
                        has_notes: draft_slide.slide.notes.is_some(),
                    })
                    .collect()
            };

            let total_slides = slides.len();

            let output = PptxListSlidesOutput {
                deck_id: args.deck_id,
                slides,
                total_slides,
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

/// Format a preview of slide content.
fn format_content_preview(content: &[crate::infrastructure::tools::documents::shared::SlideContent]) -> String {
    let mut preview = String::new();
    for item in content.iter().take(5) {
        match item {
            crate::infrastructure::tools::documents::shared::SlideContent::Title => {
                preview.push_str("[Title] ");
            }
            crate::infrastructure::tools::documents::shared::SlideContent::Subtitle => {
                preview.push_str("[Subtitle] ");
            }
            crate::infrastructure::tools::documents::shared::SlideContent::TextBox { text, .. } => {
                preview.push_str(&format!("Text: {} ", truncate(text, 30)));
            }
            crate::infrastructure::tools::documents::shared::SlideContent::BulletList { items, .. } => {
                preview.push_str(&format!("Bullets: {} ", items.len()));
            }
            crate::infrastructure::tools::documents::shared::SlideContent::NumberedList { items } => {
                preview.push_str(&format!("Numbered: {} ", items.len()));
            }
            crate::infrastructure::tools::documents::shared::SlideContent::Table { headers, .. } => {
                preview.push_str(&format!("Table: {} cols ", headers.len()));
            }
            crate::infrastructure::tools::documents::shared::SlideContent::Image { path, .. } => {
                preview.push_str(&format!("[Image: {}] ", truncate(path, 20)));
            }
            crate::infrastructure::tools::documents::shared::SlideContent::ChartPlaceholder {
                title,
                ..
            } => {
                preview.push_str(&format!("[Chart: {}] ", truncate(title, 20)));
            }
        }
        if preview.len() > 100 {
            break;
        }
    }
    truncate(&preview, 100)
}

/// Truncate a string to a maximum length.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

// ============================================================================
// Get Slide
// ============================================================================

/// Arguments for getting a slide.
#[derive(Debug, Deserialize)]
pub struct PptxGetSlideArgs {
    /// Deck ID from pptx_start
    pub deck_id: String,
    /// Slide number (1-based) or slide_id
    pub slide: String,
    /// Whether to include notes
    #[serde(default)]
    pub include_notes: bool,
}

/// Output from getting a slide.
#[derive(Debug, Serialize)]
pub struct PptxGetSlideOutput {
    /// Slide ID
    pub slide_id: String,
    /// Slide number
    pub slide_number: usize,
    /// Layout
    pub layout: String,
    /// Title
    pub title: String,
    /// Content items
    pub content: Vec<serde_json::Value>,
    /// Speaker notes (if requested and present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Tool to get a specific slide from a PPTX presentation.
pub struct PptxGetSlideTool;

impl Tool for PptxGetSlideTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("pptx", "get_slide"),
            description: "Get a specific slide from a PowerPoint presentation. You can reference the slide by number (1-based) or by slide_id.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "deck_id": {
                        "type": "string",
                        "description": "Deck ID from pptx_start"
                    },
                    "slide": {
                        "type": "string",
                        "description": "Slide number (1-based) or slide_id"
                    },
                    "include_notes": {
                        "type": "boolean",
                        "description": "Whether to include speaker notes",
                        "default": false
                    }
                },
                "required": ["deck_id", "slide"]
            }),
            requires_workspace: false,
            experimental: false,
            feature_flag: None,
        }
    }

    fn execute(
        &self,
        _context: ToolExecutionContext,
        arguments: JsonValue,
        _app_handle: AppHandle,
        _workspace_root: Option<PathBuf>,
    ) -> ToolFuture {
        Box::pin(async move {
            let args: PptxGetSlideArgs = parse_args("pptx_get_slide", arguments)?;

            let draft_store = get_draft_store();
            let output = {
                let store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {}", e))
                })?;

                let deck = store
                    .get_pptx_draft(&args.deck_id)
                    .ok_or_else(|| ToolError::failed(format!("Deck not found: {}", args.deck_id)))?;

                // Find slide by number or ID
                let draft_slide = if let Ok(num) = args.slide.parse::<usize>() {
                    // 1-based index
                    let index = num.saturating_sub(1);
                    deck.slides.get(index).ok_or_else(|| {
                        ToolError::failed(format!("Slide number {} not found", num))
                    })?
                } else {
                    deck.slides
                        .iter()
                        .find(|s| s.id == args.slide)
                        .ok_or_else(|| {
                            ToolError::failed(format!("Slide ID {} not found", args.slide))
                        })?
                };

                // Convert content to JSON-serializable format
                let content_json: Vec<serde_json::Value> = draft_slide
                    .slide
                    .content
                    .iter()
                    .map(|c| serde_json::to_value(c).unwrap_or(serde_json::json!({})))
                    .collect();

                PptxGetSlideOutput {
                    slide_id: draft_slide.id.clone(),
                    slide_number: draft_slide.index + 1,
                    layout: format!("{:?}", draft_slide.slide.layout),
                    title: draft_slide.slide.title.clone(),
                    content: content_json,
                    notes: if args.include_notes {
                        draft_slide.slide.notes.clone()
                    } else {
                        None
                    },
                }
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

// ============================================================================
// Search Slides
// ============================================================================

/// Arguments for searching slides.
#[derive(Debug, Deserialize)]
pub struct PptxSearchSlidesArgs {
    /// Deck ID from pptx_start
    pub deck_id: String,
    /// Search query
    pub query: String,
    /// Whether to search in notes
    #[serde(default)]
    pub search_in_notes: bool,
}

/// Output from searching slides.
#[derive(Debug, Serialize)]
pub struct PptxSearchSlidesOutput {
    /// Deck ID
    pub deck_id: String,
    /// Matching slides
    pub matches: Vec<SlideMatch>,
    /// Total match count
    pub total_matches: usize,
}

/// A slide that matches the search query.
#[derive(Debug, Serialize)]
pub struct SlideMatch {
    /// Slide ID
    pub slide_id: String,
    /// Slide number
    pub slide_number: usize,
    /// Title
    pub title: String,
    /// Matched context (where the query was found)
    pub matched_in: Vec<String>,
}

/// Tool to search slides in a PPTX presentation.
pub struct PptxSearchSlidesTool;

impl Tool for PptxSearchSlidesTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("pptx", "search_slides"),
            description: "Search for slides by title or content. Returns slides that contain the search query in their title, content, or optionally notes.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "deck_id": {
                        "type": "string",
                        "description": "Deck ID from pptx_start"
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "search_in_notes": {
                        "type": "boolean",
                        "description": "Whether to search in speaker notes",
                        "default": false
                    }
                },
                "required": ["deck_id", "query"]
            }),
            requires_workspace: false,
            experimental: false,
            feature_flag: None,
        }
    }

    fn execute(
        &self,
        _context: ToolExecutionContext,
        arguments: JsonValue,
        _app_handle: AppHandle,
        _workspace_root: Option<PathBuf>,
    ) -> ToolFuture {
        Box::pin(async move {
            let args: PptxSearchSlidesArgs = parse_args("pptx_search_slides", arguments)?;
            let query_lower = args.query.to_lowercase();

            let draft_store = get_draft_store();
            let matches: Vec<SlideMatch> = {
                let store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {}", e))
                })?;

                let deck = store
                    .get_pptx_draft(&args.deck_id)
                    .ok_or_else(|| ToolError::failed(format!("Deck not found: {}", args.deck_id)))?;

                deck.slides
                    .iter()
                    .filter(|draft_slide| {
                        let slide = &draft_slide.slide;
                        let mut matched_in = Vec::new();

                        // Check title
                        if slide.title.to_lowercase().contains(&query_lower) {
                            matched_in.push("title".to_string());
                        }

                        // Check content
                        for content in &slide.content {
                            if content_contains_query(content, &query_lower) {
                                matched_in.push("content".to_string());
                                break;
                            }
                        }

                        // Check notes if requested
                        if args.search_in_notes {
                            if let Some(notes) = &slide.notes {
                                if notes.to_lowercase().contains(&query_lower) {
                                    matched_in.push("notes".to_string());
                                }
                            }
                        }

                        !matched_in.is_empty()
                    })
                    .map(|draft_slide| {
                        let slide = &draft_slide.slide;
                        let mut matched_in = Vec::new();

                        if slide.title.to_lowercase().contains(&query_lower) {
                            matched_in.push("title".to_string());
                        }

                        for content in &slide.content {
                            if content_contains_query(content, &query_lower) {
                                matched_in.push("content".to_string());
                                break;
                            }
                        }

                        if args.search_in_notes {
                            if let Some(notes) = &slide.notes {
                                if notes.to_lowercase().contains(&query_lower) {
                                    matched_in.push("notes".to_string());
                                }
                            }
                        }

                        SlideMatch {
                            slide_id: draft_slide.id.clone(),
                            slide_number: draft_slide.index + 1,
                            title: slide.title.clone(),
                            matched_in,
                        }
                    })
                    .collect()
            };

            let total_matches = matches.len();

            let output = PptxSearchSlidesOutput {
                deck_id: args.deck_id,
                matches,
                total_matches,
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

/// Check if content contains the search query.
fn content_contains_query(
    content: &crate::infrastructure::tools::documents::shared::SlideContent,
    query: &str,
) -> bool {
    match content {
        crate::infrastructure::tools::documents::shared::SlideContent::Title => false,
        crate::infrastructure::tools::documents::shared::SlideContent::Subtitle => false,
        crate::infrastructure::tools::documents::shared::SlideContent::TextBox { text, .. } => {
            text.to_lowercase().contains(query)
        }
        crate::infrastructure::tools::documents::shared::SlideContent::BulletList { items, .. } => {
            items.iter().any(|i| i.to_lowercase().contains(query))
        }
        crate::infrastructure::tools::documents::shared::SlideContent::NumberedList { items } => {
            items.iter().any(|i| i.to_lowercase().contains(query))
        }
        crate::infrastructure::tools::documents::shared::SlideContent::Table { headers, rows } => {
            headers.iter().any(|h| h.to_lowercase().contains(query))
                || rows.iter().any(|row| {
                    row.iter().any(|cell| cell.to_lowercase().contains(query))
                })
        }
        crate::infrastructure::tools::documents::shared::SlideContent::Image { path, .. } => {
            path.to_lowercase().contains(query)
        }
        crate::infrastructure::tools::documents::shared::SlideContent::ChartPlaceholder {
            title, ..
        } => title.to_lowercase().contains(query),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "he...");
        assert_eq!(truncate("hi", 2), "hi");
    }
}
