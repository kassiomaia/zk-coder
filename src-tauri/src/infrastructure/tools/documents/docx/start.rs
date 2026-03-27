//! Start a new DOCX document for incremental building.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::{DraftStore, DraftStoreExt},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tauri::AppHandle;

/// Arguments for starting a new DOCX document.
#[derive(Debug, Deserialize)]
pub struct DocxStartArgs {
    /// Output filename (with or without .docx extension)
    pub filename: String,
    /// Document title
    pub title: String,
    /// Optional template name to use
    #[serde(default)]
    pub template: Option<String>,
}

/// Output from starting a DOCX document.
#[derive(Debug, Serialize)]
pub struct DocxStartOutput {
    /// Document ID for subsequent operations
    pub document_id: String,
    /// Filename (with .docx extension guaranteed)
    pub filename: String,
    /// Draft status
    pub status: String,
    /// ISO 8601 timestamp
    pub created_at: String,
}

/// Tool to start a new DOCX document.
pub struct DocxStartTool;

impl Tool for DocxStartTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("docx", "start"),
            description: "Start a new Word document for incremental building. Returns a document_id to use with subsequent docx_add_section calls.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "filename": {
                        "type": "string",
                        "description": "Output filename (e.g., 'report.docx' or 'report')"
                    },
                    "title": {
                        "type": "string",
                        "description": "Document title"
                    },
                    "template": {
                        "type": "string",
                        "description": "Optional template name to use as base"
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
        _context: ToolExecutionContext,
        arguments: JsonValue,
        _app_handle: AppHandle,
        _workspace_root: Option<std::path::PathBuf>,
    ) -> ToolFuture {
        Box::pin(async move {
            let args: DocxStartArgs = parse_args("docx_start", arguments)?;

            // Get the global draft store
            let draft_store = DraftStore::get_draft_store();

            // Ensure filename has .docx extension
            let filename = if args.filename.ends_with(".docx") {
                args.filename
            } else {
                format!("{}.docx", args.filename)
            };

            // Create the draft
            let mut store = draft_store.lock().map_err(|e| {
                ToolError::failed(format!("Failed to acquire draft store lock: {}", e))
            })?;

            let document_id = store.create_docx_draft(filename.clone());

            // Get draft to get created_at timestamp
            let draft = store.get_docx_draft(&document_id).unwrap();

            let output = DocxStartOutput {
                document_id,
                filename,
                status: format!("{:?}", draft.status),
                created_at: draft.created_at.to_rfc3339(),
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parsing() {
        let args = serde_json::json!({
            "filename": "test.docx",
            "title": "Test Document"
        });

        let parsed: DocxStartArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.filename, "test.docx");
        assert_eq!(parsed.title, "Test Document");
        assert!(parsed.template.is_none());
    }

    #[test]
    fn test_args_without_extension() {
        let args = serde_json::json!({
            "filename": "report",
            "title": "Report"
        });

        let parsed: DocxStartArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.filename, "report");
    }

    #[test]
    fn test_args_with_template() {
        let args = serde_json::json!({
            "filename": "report.docx",
            "title": "Report",
            "template": "weekly-report"
        });

        let parsed: DocxStartArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.template, Some("weekly-report".to_string()));
    }

    #[test]
    fn test_output_serialization() {
        let output = DocxStartOutput {
            document_id: "docx_123".to_string(),
            filename: "test.docx".to_string(),
            status: "Building".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["document_id"], "docx_123");
        assert_eq!(json["filename"], "test.docx");
        assert_eq!(json["status"], "Building");
    }
}
