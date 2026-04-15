//! List, get, and search pages in a PDF document.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::get_draft_store,
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use crate::infrastructure::utils::truncate_text;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use tauri::AppHandle;

// ============================================================================
// List Pages
// ============================================================================

/// Arguments for listing pages.
#[derive(Debug, Deserialize)]
pub struct PdfListPagesArgs {
    /// PDF ID from `pdf_start`
    pub pdf_id: String,
    /// Whether to include full content (default: false)
    #[serde(default)]
    pub include_content: bool,
}

/// Output from listing pages.
#[derive(Debug, Serialize)]
pub struct PdfListPagesOutput {
    /// PDF ID
    pub pdf_id: String,
    /// List of pages
    pub pages: Vec<PageSummary>,
    /// Total page count
    pub total_pages: usize,
}

/// Summary of a page.
#[derive(Debug, Serialize)]
pub struct PageSummary {
    /// Page ID
    pub page_id: String,
    /// Page number (1-based)
    pub page_number: usize,
    /// Content type
    pub content_type: String,
    /// Content preview (first 100 chars)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

/// Tool to list pages in a PDF document.
pub struct PdfListPagesTool;

impl Tool for PdfListPagesTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("pdf", "list_pages"),
            description: "List all pages in a PDF document, showing content types and previews. Use this to understand the document structure before making modifications.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "pdf_id": {
                        "type": "string",
                        "description": "PDF ID from pdf_start"
                    },
                    "include_content": {
                        "type": "boolean",
                        "description": "Whether to include full page content (default: false)",
                        "default": false
                    }
                },
                "required": ["pdf_id"]
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
            let args: PdfListPagesArgs = parse_args("pdf_list_pages", arguments)?;

            let draft_store = get_draft_store();
            let pages: Vec<PageSummary> = {
                let store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
                })?;

                let draft = store
                    .get_pdf_draft(&args.pdf_id)
                    .ok_or_else(|| ToolError::failed(format!("PDF not found: {}", args.pdf_id)))?;

                draft
                    .pages
                    .iter()
                    .map(|draft_page| {
                        let (content_type, preview) = if args.include_content {
                            let content_str = format_content_content(&draft_page.content);
                            let type_name = get_content_type_name(&draft_page.content);
                            (type_name, Some(content_str))
                        } else {
                            let type_name = get_content_type_name(&draft_page.content);
                            let preview = Some(format_content_preview(&draft_page.content));
                            (type_name, preview)
                        };

                        PageSummary {
                            page_id: draft_page.id.clone(),
                            page_number: draft_page.index + 1,
                            content_type,
                            preview,
                        }
                    })
                    .collect()
            };

            let total_pages = pages.len();

            let output = PdfListPagesOutput {
                pdf_id: args.pdf_id,
                pages,
                total_pages,
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

/// Get the content type name.
fn get_content_type_name(
    content: &crate::infrastructure::tools::documents::shared::PdfPageContent,
) -> String {
    use crate::infrastructure::tools::documents::shared::PdfPageContent;
    match content {
        PdfPageContent::Markdown { .. } => "markdown",
        PdfPageContent::Text { .. } => "text",
        PdfPageContent::Image { .. } => "image",
    }
    .to_string()
}

/// Format content for display.
fn format_content_content(
    content: &crate::infrastructure::tools::documents::shared::PdfPageContent,
) -> String {
    use crate::infrastructure::tools::documents::shared::PdfPageContent;
    match content {
        PdfPageContent::Markdown { content } | PdfPageContent::Text { content } => content.clone(),
        PdfPageContent::Image { path, caption } => {
            format!(
                "[Image: {}]{}",
                path,
                caption
                    .as_ref()
                    .map(|c| format!(" - {c}"))
                    .unwrap_or_default()
            )
        }
    }
}

/// Format a preview of page content.
fn format_content_preview(
    content: &crate::infrastructure::tools::documents::shared::PdfPageContent,
) -> String {
    use crate::infrastructure::tools::documents::shared::PdfPageContent;
    match content {
        PdfPageContent::Markdown { content } | PdfPageContent::Text { content } => {
            truncate(content, 100)
        }
        PdfPageContent::Image { path, caption } => {
            format!(
                "[Image: {}]{}",
                truncate(path, 50),
                caption
                    .as_ref()
                    .map(|c| format!(" - {c}"))
                    .unwrap_or_default()
            )
        }
    }
}

/// Truncate a string to a maximum length.
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        truncate_text(s, max_len.saturating_sub(3))
    }
}

// ============================================================================
// Get Page
// ============================================================================

/// Arguments for getting a page.
#[derive(Debug, Deserialize)]
pub struct PdfGetPageArgs {
    /// PDF ID from `pdf_start`
    pub pdf_id: String,
    /// Page number (1-based) or `page_id`
    pub page: String,
}

/// Output from getting a page.
#[derive(Debug, Serialize)]
pub struct PdfGetPageOutput {
    /// Page ID
    pub page_id: String,
    /// Page number
    pub page_number: usize,
    /// Content type
    pub content_type: String,
    /// Page content
    pub content: serde_json::Value,
}

/// Tool to get a specific page from a PDF document.
pub struct PdfGetPageTool;

impl Tool for PdfGetPageTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("pdf", "get_page"),
            description: "Get a specific page from a PDF document. You can reference the page by number (1-based) or by page_id.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "pdf_id": {
                        "type": "string",
                        "description": "PDF ID from pdf_start"
                    },
                    "page": {
                        "type": "string",
                        "description": "Page number (1-based) or page_id"
                    }
                },
                "required": ["pdf_id", "page"]
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
            let args: PdfGetPageArgs = parse_args("pdf_get_page", arguments)?;

            let draft_store = get_draft_store();
            let output = {
                let store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
                })?;

                let draft = store
                    .get_pdf_draft(&args.pdf_id)
                    .ok_or_else(|| ToolError::failed(format!("PDF not found: {}", args.pdf_id)))?;

                // Find page by number or ID
                let draft_page = if let Ok(num) = args.page.parse::<usize>() {
                    // 1-based index
                    let index = num.saturating_sub(1);
                    draft
                        .pages
                        .get(index)
                        .ok_or_else(|| ToolError::failed(format!("Page number {num} not found")))?
                } else {
                    draft
                        .pages
                        .iter()
                        .find(|p| p.id == args.page)
                        .ok_or_else(|| {
                            ToolError::failed(format!("Page ID {} not found", args.page))
                        })?
                };

                // Convert content to JSON-serializable format
                let content_json = serde_json::to_value(&draft_page.content).unwrap_or_default();
                let content_type = get_content_type_name(&draft_page.content);

                PdfGetPageOutput {
                    page_id: draft_page.id.clone(),
                    page_number: draft_page.index + 1,
                    content_type,
                    content: content_json,
                }
            };

            Ok(serde_json::to_value(output)?)
        })
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

    #[test]
    fn test_get_content_type_name() {
        use crate::infrastructure::tools::documents::shared::PdfPageContent;

        let markdown = PdfPageContent::Markdown {
            content: "# Test".to_string(),
        };
        assert_eq!(get_content_type_name(&markdown), "markdown");

        let text = PdfPageContent::Text {
            content: "Test".to_string(),
        };
        assert_eq!(get_content_type_name(&text), "text");

        let image = PdfPageContent::Image {
            path: "/test.png".to_string(),
            caption: None,
        };
        assert_eq!(get_content_type_name(&image), "image");
    }

    #[test]
    fn test_truncate_handles_multibyte_boundary() {
        let text = format!("{}{}{}", "a".repeat(96), "─", "b".repeat(10));

        let truncated = truncate(&text, 100);

        assert!(truncated.ends_with("..."));
        assert!(truncated.starts_with(&"a".repeat(96)));
    }
}
