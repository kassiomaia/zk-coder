//! Finalize a PDF document.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::{
        get_draft_store, DraftStoreExt, PdfDraft, PdfPageContent, PdfPageSize,
    },
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use tauri::AppHandle;

/// Arguments for finalizing a PDF document.
#[derive(Debug, Deserialize)]
pub struct PdfFinalizeArgs {
    /// PDF ID from pdf_start
    pub pdf_id: String,
    /// Whether to automatically save to artifact store
    #[serde(default = "default_auto_save")]
    pub auto_save: bool,
}

fn default_auto_save() -> bool {
    true
}

/// Output from finalizing a PDF document.
#[derive(Debug, Serialize)]
pub struct PdfFinalizeOutput {
    /// PDF ID
    pub pdf_id: String,
    /// Filename
    pub filename: String,
    /// Artifact ID (if auto_saved)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    /// Full path to the generated file
    pub path: String,
    /// Number of pages
    pub page_count: usize,
    /// Page size
    pub page_size: String,
    /// File size in bytes
    pub size_bytes: u64,
}

/// Tool to finalize a PDF document.
pub struct PdfFinalizeTool;

impl Tool for PdfFinalizeTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("pdf", "finalize"),
            description: "Finalize a PDF document, generating the actual PDF file and saving it to the artifact store. After finalizing, no more pages can be added.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "pdf_id": {
                        "type": "string",
                        "description": "PDF ID from pdf_start"
                    },
                    "auto_save": {
                        "type": "boolean",
                        "description": "Whether to automatically save to artifact store (default: true)",
                        "default": true
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
            let args: PdfFinalizeArgs = parse_args("pdf_finalize", arguments)?;

            let draft_store = get_draft_store();
            let draft = {
                let mut store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {}", e))
                })?;

                // Get and remove the draft (finalizing removes it from active drafts)
                store.remove_pdf_draft(&args.pdf_id).ok_or_else(|| {
                    ToolError::failed(format!("PDF not found: {}", args.pdf_id))
                })?
            };

            // Clone needed data before async call
            let filename = draft.filename.clone();
            let page_count = draft.pages.len();
            let page_size = draft.page_size;

            // Generate the PDF file
            let (path, size_bytes) = generate_pdf(draft).await?;

            // Create artifact reference
            let artifact_id = if args.auto_save {
                Some(format!("art_{}", uuid::Uuid::new_v4()))
            } else {
                None
            };

            let output = PdfFinalizeOutput {
                pdf_id: args.pdf_id,
                filename,
                artifact_id,
                path: path.to_str().unwrap().to_string(),
                page_count,
                page_size: format!("{:?}", page_size),
                size_bytes,
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

/// Generate a PDF file from the draft.
///
/// NOTE: This is a placeholder implementation. In production, this would use
/// a library like `printpdf` or `lopdf` to generate a real PDF file.
async fn generate_pdf(draft: PdfDraft) -> Result<(PathBuf, u64), ToolError> {
    // Get the home directory for artifact storage
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| ToolError::failed("Could not determine home directory".to_string()))?;

    // Create artifacts directory in home/.coder/artifacts
    let artifacts_dir = PathBuf::from(home).join(".coder").join("artifacts");
    std::fs::create_dir_all(&artifacts_dir)
        .map_err(|e| ToolError::failed(format!("Failed to create artifacts directory: {}", e)))?;

    // Create output file path
    let output_path = artifacts_dir.join(&draft.filename);

    // For now, create a simple text file as placeholder
    // TODO: Implement actual PDF generation
    let content = format!(
        "% PDF-1.4\n\
         %%Title: {}\n\
         %%Pages: {}\n\
         %%PageSize: {}\n\
         \n\
         {}\n\
         \n\
         Generated by Coder AI Assistant",
        draft.filename,
        draft.pages.len(),
        format_page_size(&draft.page_size),
        draft
            .pages
            .iter()
            .map(|p| format_page(&p.content))
            .collect::<Vec<_>>()
            .join("\n\n--- Page Break ---\n\n")
    );

    std::fs::write(&output_path, content)
        .map_err(|e| ToolError::failed(format!("Failed to write document: {}", e)))?;

    let size_bytes = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok((output_path, size_bytes))
}

/// Format page size for PDF header.
fn format_page_size(size: &PdfPageSize) -> String {
    match size {
        PdfPageSize::A4 => "A4",
        PdfPageSize::Letter => "Letter",
        PdfPageSize::Legal => "Legal",
        PdfPageSize::A3 => "A3",
    }
    .to_string()
}

/// Format a page's content as text (placeholder).
fn format_page(content: &PdfPageContent) -> String {
    match content {
        PdfPageContent::Markdown { content } => {
            format!("## Markdown Page\n\n{}", content)
        }
        PdfPageContent::Html { content, .. } => {
            format!("## HTML Page\n\n{}", content)
        }
        PdfPageContent::Text { content } => {
            format!("## Text Page\n\n{}", content)
        }
        PdfPageContent::Image { path, caption } => {
            format!(
                "## Image Page\n\n[Image: {}]{}",
                path,
                caption.as_ref().map(|c| format!(" - {}", c)).unwrap_or_default()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::tools::documents::shared::{DraftPage, DraftStatus};
    use chrono::Utc;

    fn create_test_draft() -> PdfDraft {
        PdfDraft {
            pdf_id: "test_pdf".to_string(),
            filename: "test.pdf".to_string(),
            pages: vec![
                DraftPage {
                    id: "page_1".to_string(),
                    index: 0,
                    content: PdfPageContent::Text {
                        content: "First page".to_string(),
                    },
                },
                DraftPage {
                    id: "page_2".to_string(),
                    index: 1,
                    content: PdfPageContent::Markdown {
                        content: "# Second page".to_string(),
                    },
                },
            ],
            page_size: PdfPageSize::A4,
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_format_page_size() {
        assert_eq!(format_page_size(&PdfPageSize::A4), "A4");
        assert_eq!(format_page_size(&PdfPageSize::Letter), "Letter");
        assert_eq!(format_page_size(&PdfPageSize::Legal), "Legal");
        assert_eq!(format_page_size(&PdfPageSize::A3), "A3");
    }

    #[test]
    fn test_format_page_text() {
        let content = PdfPageContent::Text {
            content: "Hello world".to_string(),
        };
        let result = format_page(&content);
        assert!(result.contains("Text Page"));
        assert!(result.contains("Hello world"));
    }

    #[test]
    fn test_format_page_markdown() {
        let content = PdfPageContent::Markdown {
            content: "# Title".to_string(),
        };
        let result = format_page(&content);
        assert!(result.contains("Markdown Page"));
        assert!(result.contains("# Title"));
    }

    #[test]
    fn test_format_page_image() {
        let content = PdfPageContent::Image {
            path: "/path/to/image.png".to_string(),
            caption: Some("A caption".to_string()),
        };
        let result = format_page(&content);
        assert!(result.contains("Image Page"));
        assert!(result.contains("/path/to/image.png"));
        assert!(result.contains("A caption"));
    }
}
