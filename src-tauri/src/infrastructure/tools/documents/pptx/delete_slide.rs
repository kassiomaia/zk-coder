//! Delete a slide from a PPTX presentation.

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

/// Arguments for deleting a slide.
#[derive(Debug, Deserialize)]
pub struct PptxDeleteSlideArgs {
    /// Deck ID from pptx_start
    pub deck_id: String,
    /// Slide number (1-based) or slide_id
    pub slide: String,
}

/// Output from deleting a slide.
#[derive(Debug, Serialize)]
pub struct PptxDeleteSlideOutput {
    /// Deck ID
    pub deck_id: String,
    /// Slide number that was deleted
    pub deleted_slide_number: usize,
    /// New total slide count
    pub new_slide_count: usize,
    /// Confirmation message
    pub message: String,
}

/// Tool to delete a slide from a PPTX presentation.
pub struct PptxDeleteSlideTool;

impl Tool for PptxDeleteSlideTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("pptx", "delete_slide"),
            description: "Delete a slide from a PowerPoint presentation. The slide is removed and subsequent slides shift up.",
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
            let args: PptxDeleteSlideArgs = parse_args("pptx_delete_slide", arguments)?;

            let draft_store = get_draft_store();
            let (deleted_slide_number, new_slide_count, slide_title) = {
                let mut store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {}", e))
                })?;

                let deck = store.get_pptx_draft_mut(&args.deck_id).ok_or_else(|| {
                    ToolError::failed(format!("Deck not found: {}", args.deck_id))
                })?;

                // Find slide by number or ID
                let slide_index = if let Ok(num) = args.slide.parse::<usize>() {
                    // 1-based index
                    let index = num.saturating_sub(1);
                    if index >= deck.slides.len() {
                        return Err(ToolError::failed(format!("Slide number {} not found", num)));
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

                let slide_title = deck.slides[slide_index].slide.title.clone();
                let deleted_slide_number = slide_index + 1;

                // Remove the slide
                deck.slides.remove(slide_index);

                // Update indices for remaining slides
                for (i, slide) in deck.slides.iter_mut().enumerate() {
                    slide.index = i;
                }

                deck.updated_at = chrono::Utc::now();

                let new_slide_count = deck.slides.len();

                (deleted_slide_number, new_slide_count, slide_title)
            };

            let output = PptxDeleteSlideOutput {
                deck_id: args.deck_id,
                deleted_slide_number,
                new_slide_count,
                message: format!(
                    "Slide {} (\"{}\") deleted. {} slides remain.",
                    deleted_slide_number, slide_title, new_slide_count
                ),
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}
