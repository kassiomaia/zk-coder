//! Update a slide in a PPTX presentation.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::{get_draft_store, SlideLayout},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use tauri::AppHandle;

// Reuse the SlideContentItem from add_slide
use super::add_slide::SlideContentItem;

/// Arguments for updating a slide.
#[derive(Debug, Deserialize)]
pub struct PptxUpdateSlideArgs {
    /// Deck ID from `pptx_start`
    pub deck_id: String,
    /// Slide number (1-based) or `slide_id`
    pub slide: String,
    /// New slide layout (optional)
    #[serde(default)]
    pub layout: Option<String>,
    /// New slide title (optional)
    #[serde(default)]
    pub title: Option<String>,
    /// New slide content (optional)
    #[serde(default)]
    pub content: Option<Vec<SlideContentItem>>,
    /// New speaker notes (optional, set to null to clear)
    #[serde(default)]
    pub notes: Option<Option<String>>,
}

/// Output from updating a slide.
#[derive(Debug, Serialize)]
pub struct PptxUpdateSlideOutput {
    /// Slide ID
    pub slide_id: String,
    /// Slide number
    pub slide_number: usize,
    /// Confirmation message
    pub message: String,
}

/// Tool to update a slide in a PPTX presentation.
pub struct PptxUpdateSlideTool;

impl Tool for PptxUpdateSlideTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("pptx", "update_slide"),
            description: "Update an existing slide in a PowerPoint presentation. You can modify the layout, title, content, and notes. Only include the fields you want to change.",
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
                    "layout": {
                        "type": "string",
                        "description": "New slide layout"
                    },
                    "title": {
                        "type": "string",
                        "description": "New slide title"
                    },
                    "content": {
                        "type": "array",
                        "description": "New slide content items"
                    },
                    "notes": {
                        "type": "string",
                        "description": "New speaker notes (set to null to clear)"
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
            let args: PptxUpdateSlideArgs = parse_args("pptx_update_slide", arguments)?;

            // Check if nothing to update
            if args.layout.is_none()
                && args.title.is_none()
                && args.content.is_none()
                && args.notes.is_none()
            {
                return Err(ToolError::failed(
                    "No updates specified. Provide at least one field to update.".to_string(),
                ));
            }

            let draft_store = get_draft_store();
            let (slide_id, slide_number, slide_title) = {
                let mut store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
                })?;

                let deck = store.get_pptx_draft_mut(&args.deck_id).ok_or_else(|| {
                    ToolError::failed(format!("Deck not found: {}", args.deck_id))
                })?;

                // Find slide by number or ID
                let slide_index = if let Ok(num) = args.slide.parse::<usize>() {
                    // 1-based index
                    let index = num.saturating_sub(1);
                    if index >= deck.slides.len() {
                        return Err(ToolError::failed(format!("Slide number {num} not found")));
                    }
                    index
                } else {
                    deck.slides
                        .iter()
                        .position(|s| s.id == args.slide)
                        .ok_or_else(|| {
                            ToolError::failed(format!("Slide ID {} not found", args.slide))
                        })?
                };

                let draft_slide = &mut deck.slides[slide_index];

                // Update layout if provided
                if let Some(layout_str) = &args.layout {
                    draft_slide.slide.layout = parse_layout(layout_str);
                }

                // Update title if provided
                if let Some(title) = &args.title {
                    draft_slide.slide.title = title.clone();
                }

                // Update content if provided
                if let Some(content_items) = args.content {
                    draft_slide.slide.content = content_items.into_iter().map(Into::into).collect();
                }

                // Update notes if provided (Some(None) means clear, Some(Some(s)) means set)
                if let Some(notes_opt) = args.notes {
                    draft_slide.slide.notes = notes_opt;
                }

                deck.updated_at = chrono::Utc::now();

                (
                    draft_slide.id.clone(),
                    slide_index + 1,
                    draft_slide.slide.title.clone(),
                )
            };

            let output = PptxUpdateSlideOutput {
                slide_id,
                slide_number,
                message: format!("Slide \"{slide_title}\" updated successfully"),
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

/// Parse layout string to `SlideLayout` enum.
fn parse_layout(s: &str) -> SlideLayout {
    match s.to_lowercase().as_str() {
        "title" | "titleslide" => SlideLayout::TitleSlide,
        "section" | "sectionheader" => SlideLayout::SectionHeader,
        "twocolumn" => SlideLayout::TwoColumn,
        "twocontent" => SlideLayout::TwoContent,
        "comparison" => SlideLayout::Comparison,
        "caption" | "contentwithcaption" => SlideLayout::ContentWithCaption,
        "blank" => SlideLayout::Blank,
        _ => SlideLayout::TitleAndContent,
    }
}
