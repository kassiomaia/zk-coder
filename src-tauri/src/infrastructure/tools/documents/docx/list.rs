//! List sections in a DOCX document.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::{DraftStore, DraftStoreExt},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tauri::AppHandle;

/// Arguments for listing sections in a DOCX document.
#[derive(Debug, Deserialize)]
pub struct DocxListSectionsArgs {
    /// Document ID from `docx_start`
    pub document_id: String,
    /// Whether to include full content (false = headings/structure only)
    #[serde(default = "default_include_content")]
    pub include_content: bool,
}

const fn default_include_content() -> bool {
    false
}

/// Summary of a section for listing.
#[derive(Debug, Serialize)]
pub struct SectionSummary {
    /// Section ID
    pub section_id: String,
    /// Section index in document
    pub index: usize,
    /// Content type (e.g., "Heading", "Paragraph", "Table")
    pub content_type: String,
    /// Preview of content (first 100 chars)
    pub preview: String,
    /// Heading level (if applicable)
    pub heading_level: Option<u8>,
}

/// Output from listing sections.
#[derive(Debug, Serialize)]
pub struct DocxListSectionsOutput {
    /// Document ID
    pub document_id: String,
    /// List of section summaries
    pub sections: Vec<SectionSummary>,
    /// Total number of sections
    pub total_section_count: usize,
}

/// Tool to list sections in a DOCX document.
pub struct DocxListSectionsTool;

impl Tool for DocxListSectionsTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("docx", "list_sections"),
            description: "List all sections in a DOCX document, showing the document structure. Use include_content=false for just headings/structure, or true to see full content.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "document_id": {
                        "type": "string",
                        "description": "Document ID from docx_start"
                    },
                    "include_content": {
                        "type": "boolean",
                        "description": "Whether to include full content (default: false)",
                        "default": false
                    }
                },
                "required": ["document_id"]
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
            let args: DocxListSectionsArgs = parse_args("docx_list_sections", arguments)?;

            let draft_store = DraftStore::get_draft_store();
            let store = draft_store.lock().map_err(|e| {
                ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
            })?;

            let draft = store.get_docx_draft(&args.document_id).ok_or_else(|| {
                ToolError::failed(format!("Document not found: {}", args.document_id))
            })?;

            let sections: Vec<SectionSummary> = draft
                .sections
                .iter()
                .map(|section| summarize_section(section, args.include_content))
                .collect();

            let output = DocxListSectionsOutput {
                document_id: args.document_id,
                total_section_count: draft.sections.len(),
                sections,
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

/// Create a summary of a section.
fn summarize_section(
    section: &crate::infrastructure::tools::documents::shared::DraftSection,
    include_content: bool,
) -> SectionSummary {
    use crate::infrastructure::tools::documents::shared::DocxContent;

    let (content_type, preview, heading_level) = match &section.content {
        DocxContent::Heading { level, text, .. } => (
            "Heading".to_string(),
            if include_content {
                text.clone()
            } else {
                text.clone()
            },
            Some(*level),
        ),
        DocxContent::Paragraph { text, .. } => (
            "Paragraph".to_string(),
            truncate_preview(text, include_content),
            None,
        ),
        DocxContent::BulletList { items } => (
            "BulletList".to_string(),
            if include_content {
                format!("{} items", items.len())
            } else {
                items
                    .first()
                    .map(|s| truncate_preview(s, false))
                    .unwrap_or_default()
            },
            None,
        ),
        DocxContent::NumberedList { items, .. } => (
            "NumberedList".to_string(),
            if include_content {
                format!("{} items", items.len())
            } else {
                items
                    .first()
                    .map(|s| truncate_preview(s, false))
                    .unwrap_or_default()
            },
            None,
        ),
        DocxContent::Table { headers, rows, .. } => (
            "Table".to_string(),
            if include_content {
                format!("{} x {} table", headers.len(), rows.len())
            } else {
                format!("Table ({} columns)", headers.len())
            },
            None,
        ),
        DocxContent::Image { path, .. } => (
            "Image".to_string(),
            format!(
                "Image: {}",
                std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("image")
            ),
            None,
        ),
        DocxContent::PageBreak => (
            "PageBreak".to_string(),
            "--- Page Break ---".to_string(),
            None,
        ),
        DocxContent::HorizontalRule => ("HorizontalRule".to_string(), "---".to_string(), None),
    };

    SectionSummary {
        section_id: section.id.clone(),
        index: section.index,
        content_type,
        preview,
        heading_level,
    }
}

/// Truncate text to a preview (max 100 chars).
fn truncate_preview(text: &str, full: bool) -> String {
    if full {
        text.to_string()
    } else if text.len() > 100 {
        format!("{}...", &text[..97])
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::tools::documents::shared::{DocxContent, DraftSection};

    #[test]
    fn test_summarize_heading() {
        let section = DraftSection {
            id: "sec_1".to_string(),
            index: 0,
            content: DocxContent::Heading {
                level: 2,
                text: "Introduction".to_string(),
                page_break_before: false,
            },
        };

        let summary = summarize_section(&section, false);
        assert_eq!(summary.content_type, "Heading");
        assert_eq!(summary.preview, "Introduction");
        assert_eq!(summary.heading_level, Some(2));
    }

    #[test]
    fn test_summarize_paragraph_truncated() {
        let long_text = "This is a very long paragraph that should be truncated when displayed as a preview because it exceeds one hundred characters in length.";
        let section = DraftSection {
            id: "sec_2".to_string(),
            index: 1,
            content: DocxContent::Paragraph {
                text: long_text.to_string(),
                style: None,
            },
        };

        let summary = summarize_section(&section, false);
        assert_eq!(summary.content_type, "Paragraph");
        assert!(summary.preview.len() <= 100); // Should be truncated
        assert!(summary.preview.ends_with("..."));
    }

    #[test]
    fn test_summarize_bullet_list() {
        let section = DraftSection {
            id: "sec_3".to_string(),
            index: 2,
            content: DocxContent::BulletList {
                items: vec![
                    "Item 1".to_string(),
                    "Item 2".to_string(),
                    "Item 3".to_string(),
                ],
            },
        };

        let summary = summarize_section(&section, false);
        assert_eq!(summary.content_type, "BulletList");
        assert_eq!(summary.preview, "Item 1");
    }
}
