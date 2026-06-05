//! Start a new DOCX document for incremental building.

use crate::core::documents::docx::theme_patch::DocxThemePatch;
use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::{sanitize_filename, DraftStore, DraftStoreExt},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tauri::AppHandle;

use super::schema::theme_schema;

/// Arguments for starting a new DOCX document.
#[derive(Debug, Deserialize)]
pub struct DocxStartArgs {
    /// Output filename (with or without .docx extension).
    pub filename: String,
    /// Document title. Rendered into core metadata and available to
    /// header/footer templates as `{title}`.
    pub title: String,
    /// Optional LLM-supplied styling overlay.
    #[serde(default)]
    pub theme: Option<JsonValue>,
}

/// Output from starting a DOCX document.
#[derive(Debug, Serialize)]
pub struct DocxStartOutput {
    /// Document ID for subsequent operations.
    pub document_id: String,
    /// Filename (with .docx extension guaranteed).
    pub filename: String,
    /// Draft status.
    pub status: String,
    /// ISO 8601 timestamp.
    pub created_at: String,
    /// `true` when an explicit `theme` was supplied and validated.
    pub theme_applied: bool,
}

/// Tool to start a new DOCX document.
pub struct DocxStartTool;

impl Tool for DocxStartTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("docx", "start"),
            description:
                "GENERATE a Word document (.docx). Use this to create documents from scratch. \
              Workflow: call this tool first, then add content with docx_add_section, \
              then finalize with docx_finalize. The document will be saved to the artifact store \
              and a download link will appear in the chat. Supply an optional `theme` to \
              override fonts, colors, margins, spacing, or header/footer. DO NOT write scripts \
              to generate documents.",
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
                        "description": "Document title (used for core metadata and the {title} token in header/footer templates)"
                    },
                    "theme": theme_schema()
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
            let theme_patch = decode_theme_patch(args.theme.as_ref())?;
            let theme_applied = theme_patch.is_some();

            let draft_store = DraftStore::get_draft_store();
            let filename = sanitize_filename(&args.filename, "docx");

            let mut store = draft_store.lock().map_err(|e| {
                ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
            })?;

            let document_id =
                store.create_docx_draft(filename.clone(), args.title.clone(), theme_patch);

            let draft = store.get_docx_draft(&document_id).ok_or_else(|| {
                ToolError::failed(format!(
                    "Document not found immediately after creation: {document_id}"
                ))
            })?;

            let output = DocxStartOutput {
                document_id,
                filename,
                status: format!("{:?}", draft.status),
                created_at: draft.created_at.to_rfc3339(),
                theme_applied,
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

/// Deserialise and validate a `theme` argument. `null` or missing is
/// returned as `Ok(None)`. Any validation failure surfaces as
/// [`ToolError::invalid_args`] with a dotted JSON path so the LLM can
/// self-correct.
pub(super) fn decode_theme_patch(
    raw: Option<&JsonValue>,
) -> Result<Option<DocxThemePatch>, ToolError> {
    let Some(raw) = raw else { return Ok(None) };
    if raw.is_null() {
        return Ok(None);
    }
    let patch: DocxThemePatch = serde_json::from_value(raw.clone())
        .map_err(|e| ToolError::invalid_args(format!("theme: {e}")))?;
    patch
        .validate()
        .map_err(|e| ToolError::invalid_args(e.to_string()))?;
    Ok(Some(patch))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn args_parsing_accepts_filename_and_title() {
        let args = json!({
            "filename": "test.docx",
            "title": "Test Document"
        });
        let parsed: DocxStartArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.filename, "test.docx");
        assert_eq!(parsed.title, "Test Document");
        assert!(parsed.theme.is_none());
    }

    #[test]
    fn decode_theme_patch_none() {
        assert!(decode_theme_patch(None).unwrap().is_none());
        let null = serde_json::Value::Null;
        assert!(decode_theme_patch(Some(&null)).unwrap().is_none());
    }

    #[test]
    fn decode_theme_patch_valid() {
        let j = json!({
            "typography": { "body_font_size_pt": 12 }
        });
        let patch = decode_theme_patch(Some(&j)).unwrap().unwrap();
        assert_eq!(patch.typography.unwrap().body_font_size_pt, Some(12));
    }

    #[test]
    fn decode_theme_patch_invalid_surfaces_error() {
        let j = json!({
            "typography": { "body_font_size_pt": 500 }
        });
        let err = decode_theme_patch(Some(&j)).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("body_font_size_pt"));
    }
}
