//! Start a new PDF document.

use crate::infrastructure::tools::{
    documents::shared::{get_draft_store, PdfPageSize},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use tauri::AppHandle;

/// Arguments for starting a PDF document.
#[derive(Debug, Deserialize)]
pub struct PdfStartArgs {
    /// Output filename (should end in .pdf)
    pub filename: String,
    /// Document title
    pub title: String,
    /// Page size (default: A4)
    #[serde(default)]
    pub page_size: Option<String>,
}

/// Output from starting a PDF document.
#[derive(Debug, Serialize)]
pub struct PdfStartOutput {
    /// PDF ID for subsequent operations
    pub pdf_id: String,
    /// Filename
    pub filename: String,
    /// Title
    pub title: String,
    /// Page size
    pub page_size: String,
    /// Current status (always "building" on start)
    pub status: String,
    /// Page count (always 0 on start)
    pub page_count: usize,
    /// When the document was created
    pub created_at: String,
}

/// Tool to start a new PDF document.
pub struct PdfStartTool;

impl Tool for PdfStartTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("pdf", "start"),
            description: "Create a professional PDF document. \
              Workflow: (1) call pdf_start, (2) call pdf_add_page one or more times to add content, \
              (3) call pdf_finalize to render and save the PDF. \
              The rendered PDF supports: headings (H1-H6), paragraphs with **bold**, *italic*, \
              `code`, and [links](url), ordered/unordered/nested lists, fenced code blocks \
              (```lang), pipe tables (| A | B |), blockquotes (> text) rendered as callout boxes, \
              horizontal rules (---) as page breaks, and embedded images. \
              The output includes page headers/footers with page numbers, a font-embedded \
              professional layout, and PDF bookmarks from headings. \
              DO NOT write scripts to generate PDFs — use this tool instead. \
              Never use pandoc, LaTeX, or shell commands to generate PDFs — this tool replaces all of those.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "filename": {
                        "type": "string",
                        "description": "Output filename (should end in .pdf)"
                    },
                    "title": {
                        "type": "string",
                        "description": "Document title"
                    },
                    "page_size": {
                        "type": "string",
                        "description": "Page size",
                        "enum": ["A4", "Letter", "Legal", "A3"],
                        "default": "A4"
                    }
                },
                "required": ["filename", "title"]
            }),
            requires_workspace: false,
            experimental: false,
            feature_flag: None,
        }
    }

    fn execute(
        &self,
        _context: crate::infrastructure::tools::context::ToolExecutionContext,
        arguments: JsonValue,
        _app_handle: AppHandle,
        _workspace_root: Option<PathBuf>,
    ) -> ToolFuture {
        Box::pin(async move {
            let args: PdfStartArgs = parse_args("pdf_start", arguments)?;

            // Normalize filename to ensure .pdf extension
            let filename = if args.filename.ends_with(".pdf") {
                args.filename.clone()
            } else {
                format!("{}.pdf", args.filename)
            };

            // Parse page size
            let page_size = parse_page_size(args.page_size.as_deref().unwrap_or("A4"));

            let draft_store = get_draft_store();
            let pdf_id = {
                let mut store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
                })?;

                store.create_pdf_draft(filename.clone(), args.title.clone(), Some(page_size))
            };

            let created_at = chrono::Utc::now().to_rfc3339();
            let page_size_str = format!("{page_size:?}");

            let output = PdfStartOutput {
                pdf_id,
                filename,
                title: args.title,
                page_size: page_size_str,
                status: "building".to_string(),
                page_count: 0,
                created_at,
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

/// Parse page size string to `PdfPageSize` enum.
fn parse_page_size(s: &str) -> PdfPageSize {
    match s.to_uppercase().as_str() {
        "LETTER" => PdfPageSize::Letter,
        "LEGAL" => PdfPageSize::Legal,
        "A3" => PdfPageSize::A3,
        _ => PdfPageSize::A4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_page_size() {
        assert_eq!(parse_page_size("A4"), PdfPageSize::A4);
        assert_eq!(parse_page_size("a4"), PdfPageSize::A4);
        assert_eq!(parse_page_size("Letter"), PdfPageSize::Letter);
        assert_eq!(parse_page_size("letter"), PdfPageSize::Letter);
        assert_eq!(parse_page_size("LEGAL"), PdfPageSize::Legal);
        assert_eq!(parse_page_size("A3"), PdfPageSize::A3);
        assert_eq!(parse_page_size("unknown"), PdfPageSize::A4); // default
    }

    #[test]
    fn test_args_parsing() {
        let args = serde_json::json!({
            "filename": "document.pdf",
            "title": "My Document"
        });

        let parsed: PdfStartArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.filename, "document.pdf");
        assert_eq!(parsed.title, "My Document");
        assert!(parsed.page_size.is_none());
    }

    #[test]
    fn test_args_with_page_size() {
        let args = serde_json::json!({
            "filename": "report.pdf",
            "title": "Report",
            "page_size": "Letter"
        });

        let parsed: PdfStartArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.page_size, Some("Letter".to_string()));
    }

    #[test]
    fn test_output_serialization() {
        let output = PdfStartOutput {
            pdf_id: "pdf_123".to_string(),
            filename: "test.pdf".to_string(),
            title: "Test".to_string(),
            page_size: "A4".to_string(),
            status: "Building".to_string(),
            page_count: 0,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["pdf_id"], "pdf_123");
        assert_eq!(json["filename"], "test.pdf");
        assert_eq!(json["status"], "Building");
        assert_eq!(json["page_count"], 0);
    }
}
