//! Delete a page from a PDF document.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::get_draft_store,
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use tauri::AppHandle;

/// Arguments for deleting a page.
#[derive(Debug, Deserialize)]
pub struct PdfDeletePageArgs {
    /// PDF ID from `pdf_start`
    pub pdf_id: String,
    /// Page number (1-based) or `page_id`
    pub page: String,
}

/// Output from deleting a page.
#[derive(Debug, Serialize)]
pub struct PdfDeletePageOutput {
    /// PDF ID
    pub pdf_id: String,
    /// Page number that was deleted
    pub deleted_page_number: usize,
    /// New total page count
    pub new_page_count: usize,
    /// Confirmation message
    pub message: String,
}

/// Tool to delete a page from a PDF document.
pub struct PdfDeletePageTool;

impl Tool for PdfDeletePageTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("pdf", "delete_page"),
            description: "Delete a page from a PDF document. The page is removed and subsequent pages shift up.",
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
            let args: PdfDeletePageArgs = parse_args("pdf_delete_page", arguments)?;

            let draft_store = get_draft_store();
            let (deleted_page_number, new_page_count, content_type) = {
                let mut store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
                })?;

                let draft = store
                    .get_pdf_draft_mut(&args.pdf_id)
                    .ok_or_else(|| ToolError::failed(format!("PDF not found: {}", args.pdf_id)))?;

                // Find page by number or ID
                let page_index = if let Ok(num) = args.page.parse::<usize>() {
                    // 1-based index
                    let index = num.saturating_sub(1);
                    if index >= draft.pages.len() {
                        return Err(ToolError::failed(format!("Page number {num} not found")));
                    }
                    index
                } else {
                    draft
                        .pages
                        .iter()
                        .position(|p| p.id == args.page)
                        .ok_or_else(|| {
                            ToolError::failed(format!("Page ID {} not found", args.page))
                        })?
                };

                // Get content type for message
                let content_type = get_content_type_name(&draft.pages[page_index].content);
                let deleted_page_number = page_index + 1;

                // Remove the page
                draft.pages.remove(page_index);

                // Update indices for remaining pages
                for (i, page) in draft.pages.iter_mut().enumerate() {
                    page.index = i;
                }

                draft.updated_at = chrono::Utc::now();

                let new_page_count = draft.pages.len();

                (deleted_page_number, new_page_count, content_type)
            };

            let output = PdfDeletePageOutput {
                pdf_id: args.pdf_id,
                deleted_page_number,
                new_page_count,
                message: format!(
                    "Page {deleted_page_number} ({content_type}) deleted. {new_page_count} pages remain."
                ),
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
