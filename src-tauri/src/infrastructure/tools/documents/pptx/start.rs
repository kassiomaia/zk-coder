//! Start a new PPTX presentation.

use crate::infrastructure::tools::{
    documents::shared::{get_draft_store, sanitize_filename},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use tauri::AppHandle;

/// Arguments for starting a PPTX presentation.
#[derive(Debug, Deserialize)]
pub struct PptxStartArgs {
    /// Output filename (should end in .pptx)
    pub filename: String,
    /// Presentation title
    pub title: String,
    /// Optional template to use
    #[serde(default)]
    pub template: Option<String>,
    /// Optional theme color scheme
    #[serde(default)]
    pub theme: Option<String>,
}

/// Output from starting a presentation.
#[derive(Debug, Serialize)]
pub struct PptxStartOutput {
    /// Deck ID for subsequent operations
    pub deck_id: String,
    /// Filename
    pub filename: String,
    /// Title
    pub title: String,
    /// Current status (always "building" on start)
    pub status: String,
    /// Slide count (always 0 on start)
    pub slide_count: usize,
    /// When the presentation was created
    pub created_at: String,
}

/// Tool to start a new PPTX presentation.
pub struct PptxStartTool;

impl Tool for PptxStartTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("pptx", "start"),
            description: "GENERATE a PowerPoint presentation (.pptx). Use this to create presentations from scratch. \
              Workflow: call this tool first, then add slides with pptx_add_slide, \
              then finalize with pptx_finalize. The presentation will be saved to the artifact store \
              and a download link will appear in the chat. DO NOT write scripts to generate presentations.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "filename": {
                        "type": "string",
                        "description": "Output filename (should end in .pptx)"
                    },
                    "title": {
                        "type": "string",
                        "description": "Presentation title"
                    },
                    "template": {
                        "type": "string",
                        "description": "Optional template name to use"
                    },
                    "theme": {
                        "type": "string",
                        "description": "Optional theme color scheme"
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
            let args: PptxStartArgs = parse_args("pptx_start", arguments)?;

            let filename = sanitize_filename(&args.filename, "pptx");

            let draft_store = get_draft_store();
            let deck_id = {
                let mut store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
                })?;

                store.create_pptx_draft(filename.clone())
            };

            let created_at = chrono::Utc::now().to_rfc3339();

            let output = PptxStartOutput {
                deck_id,
                filename,
                title: args.title,
                status: "building".to_string(),
                slide_count: 0,
                created_at,
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
            "filename": "presentation.pptx",
            "title": "My Presentation"
        });

        let parsed: PptxStartArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.filename, "presentation.pptx");
        assert_eq!(parsed.title, "My Presentation");
        assert!(parsed.template.is_none());
    }

    #[test]
    fn test_args_without_extension() {
        let args = serde_json::json!({
            "filename": "deck",
            "title": "Deck Title"
        });

        let parsed: PptxStartArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.filename, "deck");
    }

    #[test]
    fn test_args_with_template() {
        let args = serde_json::json!({
            "filename": "report.pptx",
            "title": "Report",
            "template": "pitch-deck"
        });

        let parsed: PptxStartArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.template, Some("pitch-deck".to_string()));
    }

    #[test]
    fn test_output_serialization() {
        let output = PptxStartOutput {
            deck_id: "pptx_123".to_string(),
            filename: "test.pptx".to_string(),
            title: "Test".to_string(),
            status: "Building".to_string(),
            slide_count: 0,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["deck_id"], "pptx_123");
        assert_eq!(json["filename"], "test.pptx");
        assert_eq!(json["status"], "Building");
        assert_eq!(json["slide_count"], 0);
    }
}
