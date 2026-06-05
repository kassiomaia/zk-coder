//! Add a section to a DOCX document.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::{DocxContent, DraftStore, DraftStoreExt},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tauri::AppHandle;

use super::schema::{section_content_schema, theme_schema};
use super::start::decode_theme_patch;

/// Arguments for adding a section to a DOCX document.
#[derive(Debug, Deserialize)]
pub struct DocxAddSectionArgs {
    /// Document ID from `docx_start`.
    pub document_id: String,
    /// Section content to add.
    pub content: DocxContent,
    /// Optional per-section styling patch. When present, opens a new
    /// OOXML section break with distinct page geometry (paper,
    /// orientation, margins) or decorations. Body-level fields
    /// (typography, palette, spacing, fonts) are ignored here — those
    /// live at the document level.
    #[serde(default)]
    pub theme_override: Option<JsonValue>,
}

/// Output from adding a section.
#[derive(Debug, Serialize)]
pub struct DocxAddSectionOutput {
    /// Section ID that was added.
    pub section_id: String,
    /// Document ID.
    pub document_id: String,
    /// Index of the section in the document.
    pub section_index: usize,
    /// Total number of sections in the document.
    pub current_section_count: usize,
    /// `true` when a `theme_override` was supplied and validated.
    pub theme_override_applied: bool,
}

/// Tool to add a section to a DOCX document.
pub struct DocxAddSectionTool;

impl Tool for DocxAddSectionTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("docx", "add_section"),
            description: "Add a section (heading, paragraph, list, table, image, page break, or rule) to a DOCX document. \
              Text fields accept either a plain string or an array of runs `[{ text, bold?, italic?, underline?, strike?, code?, color?, link? }]` \
              for inline formatting. Pass an optional `theme_override` to open a new OOXML section break with distinct page geometry. \
              Use this repeatedly to build documents incrementally.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "document_id": {
                        "type": "string",
                        "description": "Document ID from docx_start"
                    },
                    "content": section_content_schema(),
                    "theme_override": theme_schema()
                },
                "required": ["document_id", "content"]
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
            let args: DocxAddSectionArgs = parse_args("docx_add_section", arguments)?;
            let theme_override = decode_theme_patch(args.theme_override.as_ref())?;
            let theme_override_applied = theme_override.is_some();

            let draft_store = DraftStore::get_draft_store();
            let mut store = draft_store.lock().map_err(|e| {
                ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
            })?;

            let section_id = store
                .add_docx_section(&args.document_id, args.content, theme_override)
                .map_err(|e| ToolError::failed(format!("Failed to add section: {e}")))?;

            let draft = store.get_docx_draft(&args.document_id).ok_or_else(|| {
                ToolError::failed(format!(
                    "Document disappeared after section add: {}",
                    args.document_id
                ))
            })?;

            let output = DocxAddSectionOutput {
                section_id,
                document_id: args.document_id,
                section_index: draft.sections.len() - 1,
                current_section_count: draft.sections.len(),
                theme_override_applied,
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::infrastructure::tools::documents::shared::ParagraphText;
    use serde_json::json;

    #[test]
    fn heading_with_plain_text_parses() {
        let args = json!({
            "document_id": "docx_123",
            "content": {
                "type": "Heading",
                "level": 1,
                "text": "Introduction"
            }
        });
        let parsed: DocxAddSectionArgs = serde_json::from_value(args).unwrap();
        match parsed.content {
            DocxContent::Heading { level, text, .. } => {
                assert_eq!(level, 1);
                assert_eq!(text.as_plain_string(), "Introduction");
            }
            _ => panic!("expected Heading"),
        }
    }

    #[test]
    fn paragraph_with_rich_runs_parses() {
        let args = json!({
            "document_id": "docx_123",
            "content": {
                "type": "Paragraph",
                "text": [
                    { "text": "Important: ", "bold": true },
                    { "text": "read carefully." }
                ]
            }
        });
        let parsed: DocxAddSectionArgs = serde_json::from_value(args).unwrap();
        match parsed.content {
            DocxContent::Paragraph {
                text: ParagraphText::Rich(runs),
                ..
            } => {
                assert_eq!(runs.len(), 2);
                assert!(runs[0].bold);
            }
            _ => panic!("expected rich paragraph"),
        }
    }

    #[test]
    fn theme_override_accepts_partial_page_override() {
        let args = json!({
            "document_id": "docx_123",
            "content": { "type": "PageBreak" },
            "theme_override": {
                "page": { "landscape": true, "margin_top_mm": 15.0 }
            }
        });
        let parsed: DocxAddSectionArgs = serde_json::from_value(args).unwrap();
        let patch = decode_theme_patch(parsed.theme_override.as_ref())
            .unwrap()
            .unwrap();
        assert_eq!(patch.page.unwrap().landscape, Some(true));
    }
}
