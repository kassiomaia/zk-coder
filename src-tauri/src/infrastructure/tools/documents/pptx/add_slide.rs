//! Add a slide to a PPTX presentation.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::{get_draft_store, PptxSlide, SlideContent, SlideLayout},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use tauri::AppHandle;

/// Arguments for adding a slide to a presentation.
#[derive(Debug, Deserialize)]
pub struct PptxAddSlideArgs {
    /// Deck ID from `pptx_start`
    pub deck_id: String,
    /// Slide layout
    #[serde(default)]
    pub layout: String,
    /// Slide title
    pub title: String,
    /// Slide content items
    #[serde(default)]
    pub content: Vec<SlideContentItem>,
    /// Optional speaker notes
    #[serde(default)]
    pub notes: Option<String>,
    /// Position to insert (default: "end")
    #[serde(default)]
    pub position: String,
}

/// Slide content item for serialization.
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SlideContentItem {
    Title,
    Subtitle,
    TextBox {
        text: String,
        #[serde(default)]
        position: Option<String>,
    },
    BulletList {
        items: Vec<String>,
        #[serde(default)]
        level: u8,
    },
    NumberedList {
        items: Vec<String>,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Image {
        path: String,
        #[serde(default)]
        caption: Option<String>,
    },
    ChartPlaceholder {
        title: String,
        data_type: String,
    },
}

impl From<SlideContentItem> for SlideContent {
    fn from(item: SlideContentItem) -> Self {
        match item {
            SlideContentItem::Title => Self::Title,
            SlideContentItem::Subtitle => Self::Subtitle,
            SlideContentItem::TextBox { text, position } => Self::TextBox { text, position },
            SlideContentItem::BulletList { items, level } => Self::BulletList { items, level },
            SlideContentItem::NumberedList { items } => Self::NumberedList { items },
            SlideContentItem::Table { headers, rows } => Self::Table { headers, rows },
            SlideContentItem::Image { path, caption } => Self::Image { path, caption },
            SlideContentItem::ChartPlaceholder { title, data_type } => {
                Self::ChartPlaceholder { title, data_type }
            }
        }
    }
}

/// Output from adding a slide.
#[derive(Debug, Serialize)]
pub struct PptxAddSlideOutput {
    /// Slide ID
    pub slide_id: String,
    /// Deck ID
    pub deck_id: String,
    /// Slide number (1-based)
    pub slide_number: usize,
    /// Total slide count
    pub total_slides: usize,
}

/// Tool to add a slide to a PPTX presentation.
pub struct PptxAddSlideTool;

impl Tool for PptxAddSlideTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("pptx", "add_slide"),
            description: "Add a slide to a PowerPoint presentation. The slide is appended to the end of the presentation by default.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "deck_id": {
                        "type": "string",
                        "description": "Deck ID from pptx_start"
                    },
                    "layout": {
                        "type": "string",
                        "description": "Slide layout",
                        "enum": ["TitleSlide", "TitleAndContent", "SectionHeader", "TwoColumn", "TwoContent", "Comparison", "ContentWithCaption", "Blank"],
                        "default": "TitleAndContent"
                    },
                    "title": {
                        "type": "string",
                        "description": "Slide title"
                    },
                    "content": {
                        "type": "array",
                        "description": "Slide content items",
                        "items": {
                            "type": "object",
                            "oneOf": [
                                {"type": "object", "required": ["type"], "properties": {"type": {"const": "title"}}},
                                {"type": "object", "required": ["type"], "properties": {"type": {"const": "subtitle"}}},
                                {"type": "object", "required": ["type", "text"], "properties": {"type": {"const": "text_box"}, "text": {"type": "string"}, "position": {"type": "string"}}},
                                {"type": "object", "required": ["type", "items"], "properties": {"type": {"const": "bullet_list"}, "items": {"type": "array", "items": {"type": "string"}}, "level": {"type": "integer", "minimum": 0, "maximum": 5}}}
                            ]
                        }
                    },
                    "notes": {
                        "type": "string",
                        "description": "Optional speaker notes"
                    },
                    "position": {
                        "type": "string",
                        "enum": ["start", "end"],
                        "default": "end"
                    }
                },
                "required": ["deck_id", "title"]
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
            let args: PptxAddSlideArgs = parse_args("pptx_add_slide", arguments)?;

            // Parse layout string to enum
            let layout = parse_layout(&args.layout);

            // Convert content items
            let content: Vec<SlideContent> = args.content.into_iter().map(Into::into).collect();

            let slide = PptxSlide {
                layout,
                title: args.title,
                content,
                notes: args.notes,
            };

            let draft_store = get_draft_store();
            let (slide_id, slide_number, total_slides) = {
                let mut store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
                })?;

                let deck = store.get_pptx_draft(&args.deck_id).ok_or_else(|| {
                    ToolError::failed(format!("Deck not found: {}", args.deck_id))
                })?;

                let total_before = deck.slides.len();

                let slide_id = store
                    .add_pptx_slide(&args.deck_id, slide)
                    .map_err(|e| ToolError::failed(format!("Failed to add slide: {e}")))?;

                let deck = store.get_pptx_draft(&args.deck_id).ok_or_else(|| {
                    ToolError::failed(format!("Deck disappeared after slide add: {}", args.deck_id))
                })?;
                (slide_id, deck.slides.len(), total_before + 1)
            };

            let output = PptxAddSlideOutput {
                slide_id,
                deck_id: args.deck_id,
                slide_number,
                total_slides,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_layout() {
        assert_eq!(parse_layout("TitleSlide"), SlideLayout::TitleSlide);
        assert_eq!(parse_layout("title"), SlideLayout::TitleSlide);
        assert_eq!(
            parse_layout("TitleAndContent"),
            SlideLayout::TitleAndContent
        );
        assert_eq!(parse_layout("TwoColumn"), SlideLayout::TwoColumn);
        assert_eq!(parse_layout("blank"), SlideLayout::Blank);
        assert_eq!(parse_layout("unknown"), SlideLayout::TitleAndContent); // default
    }

    #[test]
    fn test_slide_content_item_conversion() {
        let item = SlideContentItem::BulletList {
            items: vec!["Item 1".to_string(), "Item 2".to_string()],
            level: 0,
        };

        let content: SlideContent = item.into();
        match content {
            SlideContent::BulletList { items, level } => {
                assert_eq!(items, vec!["Item 1", "Item 2"]);
                assert_eq!(level, 0);
            }
            _ => panic!("Expected BulletList"),
        }
    }
}
