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

/// Arguments for adding a section to a DOCX document.
#[derive(Debug, Deserialize)]
pub struct DocxAddSectionArgs {
    /// Document ID from `docx_start`
    pub document_id: String,
    /// Section content to add
    pub content: DocxContent,
}

/// Output from adding a section.
#[derive(Debug, Serialize)]
pub struct DocxAddSectionOutput {
    /// Section ID that was added
    pub section_id: String,
    /// Document ID
    pub document_id: String,
    /// Index of the section in the document
    pub section_index: usize,
    /// Total number of sections in the document
    pub current_section_count: usize,
}

/// Tool to add a section to a DOCX document.
pub struct DocxAddSectionTool;

impl Tool for DocxAddSectionTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("docx", "add_section"),
            description: "Add a section (heading, paragraph, list, table, image, or page break) to a DOCX document. Use this repeatedly to build documents incrementally.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "document_id": {
                        "type": "string",
                        "description": "Document ID from docx_start"
                    },
                    "content": {
                        "description": "Section content to add",
                        "oneOf": [
                            {
                                "type": "object",
                                "required": ["type", "level", "text", "page_break_before"],
                                "properties": {
                                    "type": { "const": "Heading" },
                                    "level": { "type": "integer", "minimum": 1, "maximum": 6 },
                                    "text": { "type": "string" },
                                    "page_break_before": { "type": "boolean" }
                                }
                            },
                            {
                                "type": "object",
                                "required": ["type", "text", "style"],
                                "properties": {
                                    "type": { "const": "Paragraph" },
                                    "text": { "type": "string" },
                                    "style": { "type": "string" }
                                }
                            },
                            {
                                "type": "object",
                                "required": ["type", "items"],
                                "properties": {
                                    "type": { "const": "BulletList" },
                                    "items": { "type": "array", "items": { "type": "string" } }
                                }
                            },
                            {
                                "type": "object",
                                "required": ["type", "items", "start_at"],
                                "properties": {
                                    "type": { "const": "NumberedList" },
                                    "items": { "type": "array", "items": { "type": "string" } },
                                    "start_at": { "type": "integer" }
                                }
                            },
                            {
                                "type": "object",
                                "required": ["type", "headers", "rows", "caption"],
                                "properties": {
                                    "type": { "const": "Table" },
                                    "headers": { "type": "array", "items": { "type": "string" } },
                                    "rows": { "type": "array", "items": { "type": "array", "items": { "type": "string" } } },
                                    "caption": { "type": "string" }
                                }
                            },
                            {
                                "type": "object",
                                "required": ["type", "path", "caption", "width"],
                                "properties": {
                                    "type": { "const": "Image" },
                                    "path": { "type": "string" },
                                    "caption": { "type": "string" },
                                    "width": { "type": "string" }
                                }
                            },
                            { "type": "object", "required": ["type"], "properties": { "type": { "const": "PageBreak" } } },
                            { "type": "object", "required": ["type"], "properties": { "type": { "const": "HorizontalRule" } } }
                        ]
                    }
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

            let draft_store = DraftStore::get_draft_store();
            let mut store = draft_store.lock().map_err(|e| {
                ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
            })?;

            let section_id = store
                .add_docx_section(&args.document_id, args.content)
                .map_err(|e| ToolError::failed(format!("Failed to add section: {e}")))?;

            let draft = store.get_docx_draft(&args.document_id).unwrap();

            let output = DocxAddSectionOutput {
                section_id,
                document_id: args.document_id,
                section_index: draft.sections.len() - 1,
                current_section_count: draft.sections.len(),
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_add_heading_args() {
        let args = json!({
            "document_id": "docx_123",
            "content": {
                "type": "Heading",
                "level": 1,
                "text": "Introduction",
                "page_break_before": false
            }
        });

        let parsed: DocxAddSectionArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.document_id, "docx_123");
        match parsed.content {
            DocxContent::Heading { level, text, .. } => {
                assert_eq!(level, 1);
                assert_eq!(text, "Introduction");
            }
            _ => panic!("Expected Heading content"),
        }
    }

    #[test]
    fn test_add_paragraph_args() {
        let args = json!({
            "document_id": "docx_123",
            "content": {
                "type": "Paragraph",
                "text": "This is a paragraph.",
                "style": null
            }
        });

        let parsed: DocxAddSectionArgs = serde_json::from_value(args).unwrap();
        match parsed.content {
            DocxContent::Paragraph { text, style } => {
                assert_eq!(text, "This is a paragraph.");
                assert!(style.is_none());
            }
            _ => panic!("Expected Paragraph content"),
        }
    }

    #[test]
    fn test_add_bullet_list_args() {
        let args = json!({
            "document_id": "docx_123",
            "content": {
                "type": "BulletList",
                "items": ["Item 1", "Item 2", "Item 3"]
            }
        });

        let parsed: DocxAddSectionArgs = serde_json::from_value(args).unwrap();
        match parsed.content {
            DocxContent::BulletList { items } => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], "Item 1");
            }
            _ => panic!("Expected BulletList content"),
        }
    }
}
