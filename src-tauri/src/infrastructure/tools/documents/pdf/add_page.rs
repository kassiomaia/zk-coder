//! Add a page to a PDF document.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::{get_draft_store, PdfInsertPosition, PdfPageContent},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use tauri::AppHandle;

/// Arguments for adding a page to a PDF.
#[derive(Debug, Deserialize)]
pub struct PdfAddPageArgs {
    /// PDF ID from `pdf_start`
    pub pdf_id: String,
    /// Page content
    pub content: PdfPageContentInput,
    /// Position to insert (default: "end")
    #[serde(default)]
    pub position: String,
}

/// Page content input for serialization.
///
/// Mirrors [`PdfPageContent`] at the tool boundary. The `html` variant was
/// removed in T1.3 — callers must emit structured Markdown or plain text.
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PdfPageContentInput {
    /// Markdown content variant.
    Markdown {
        /// The markdown source.
        content: String,
    },
    /// Plain text content variant.
    Text {
        /// The text content.
        content: String,
    },
    /// Image reference with optional caption.
    Image {
        /// Path to the image file.
        path: String,
        /// Optional caption.
        #[serde(default)]
        caption: Option<String>,
    },
}

impl From<PdfPageContentInput> for PdfPageContent {
    fn from(item: PdfPageContentInput) -> Self {
        match item {
            PdfPageContentInput::Markdown { content } => Self::Markdown { content },
            PdfPageContentInput::Text { content } => Self::Text { content },
            PdfPageContentInput::Image { path, caption } => Self::Image { path, caption },
        }
    }
}

fn parse_position(raw: &str) -> PdfInsertPosition {
    match raw.to_ascii_lowercase().as_str() {
        "start" => PdfInsertPosition::Start,
        _ => PdfInsertPosition::End,
    }
}

/// Output from adding a page.
#[derive(Debug, Serialize)]
pub struct PdfAddPageOutput {
    /// Page ID
    pub page_id: String,
    /// PDF ID
    pub pdf_id: String,
    /// Page number (1-based)
    pub page_number: usize,
    /// Total page count
    pub total_pages: usize,
}

/// Tool to add a page to a PDF document.
pub struct PdfAddPageTool;

impl Tool for PdfAddPageTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("pdf", "add_page"),
            description: "Add a page to a PDF document. Pages support Markdown, HTML, plain text, or images. The page is appended to the end by default.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "pdf_id": {
                        "type": "string",
                        "description": "PDF ID from pdf_start"
                    },
                    "content": {
                        "type": "object",
                        "description": "Page content",
                        "oneOf": [
                            {
                                "type": "object",
                                "required": ["type", "content"],
                                "properties": {
                                    "type": {"const": "markdown"},
                                    "content": {"type": "string"}
                                }
                            },
                            {
                                "type": "object",
                                "required": ["type", "content"],
                                "properties": {
                                    "type": {"const": "text"},
                                    "content": {"type": "string"}
                                }
                            },
                            {
                                "type": "object",
                                "required": ["type", "path"],
                                "properties": {
                                    "type": {"const": "image"},
                                    "path": {"type": "string"},
                                    "caption": {"type": "string"}
                                }
                            }
                        ]
                    },
                    "position": {
                        "type": "string",
                        "enum": ["start", "end"],
                        "default": "end"
                    }
                },
                "required": ["pdf_id", "content"]
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
            let args: PdfAddPageArgs = parse_args("pdf_add_page", arguments)?;

            // Convert content input to internal type
            let content: PdfPageContent = args.content.into();
            let position = parse_position(&args.position);

            let draft_store = get_draft_store();
            let (page_id, page_number, total_pages) = {
                let mut store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
                })?;

                let page_id = store
                    .add_pdf_page(&args.pdf_id, content, position)
                    .map_err(|e| ToolError::failed(format!("Failed to add page: {e}")))?;

                let draft = store
                    .get_pdf_draft(&args.pdf_id)
                    .ok_or_else(|| ToolError::failed(format!("PDF not found: {}", args.pdf_id)))?;
                let total = draft.pages.len();
                let index = draft
                    .pages
                    .iter()
                    .position(|p| p.id == page_id)
                    .unwrap_or(total.saturating_sub(1));
                (page_id, index + 1, total)
            };

            let output = PdfAddPageOutput {
                page_id,
                pdf_id: args.pdf_id,
                page_number,
                total_pages,
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_conversion() {
        let input = PdfPageContentInput::Markdown {
            content: "# Title\n\nContent".to_string(),
        };

        let content: PdfPageContent = input.into();
        match content {
            PdfPageContent::Markdown { content } => {
                assert_eq!(content, "# Title\n\nContent");
            }
            _ => panic!("Expected Markdown"),
        }
    }

    #[test]
    fn test_parse_position_defaults_to_end() {
        assert_eq!(parse_position(""), PdfInsertPosition::End);
        assert_eq!(parse_position("end"), PdfInsertPosition::End);
        assert_eq!(parse_position("END"), PdfInsertPosition::End);
        assert_eq!(parse_position("start"), PdfInsertPosition::Start);
        assert_eq!(parse_position("Start"), PdfInsertPosition::Start);
        assert_eq!(parse_position("bogus"), PdfInsertPosition::End);
    }

    #[test]
    fn test_image_content_conversion() {
        let input = PdfPageContentInput::Image {
            path: "/path/to/image.png".to_string(),
            caption: Some("A caption".to_string()),
        };

        let content: PdfPageContent = input.into();
        match content {
            PdfPageContent::Image { path, caption } => {
                assert_eq!(path, "/path/to/image.png");
                assert_eq!(caption, Some("A caption".to_string()));
            }
            _ => panic!("Expected Image"),
        }
    }
}
