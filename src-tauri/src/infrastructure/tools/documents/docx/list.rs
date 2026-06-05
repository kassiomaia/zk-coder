//! List sections in a DOCX document.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::{DocxContent, DraftSection, DraftStore, DraftStoreExt, ListItem},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use crate::infrastructure::utils::truncate_text;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tauri::AppHandle;

/// Arguments for listing sections in a DOCX document.
#[derive(Debug, Deserialize)]
pub struct DocxListSectionsArgs {
    /// Document ID from `docx_start`.
    pub document_id: String,
    /// `true` emits the full content; `false` returns a short preview.
    #[serde(default = "default_include_content")]
    pub include_content: bool,
}

const fn default_include_content() -> bool {
    false
}

/// Summary of a section for listing.
#[derive(Debug, Serialize)]
pub struct SectionSummary {
    /// Section ID.
    pub section_id: String,
    /// Section index in document.
    pub index: usize,
    /// Content type (e.g., "Heading", "Paragraph", "Table").
    pub content_type: String,
    /// Preview of content (first 100 chars unless `include_content`).
    pub preview: String,
    /// Heading level (if applicable).
    pub heading_level: Option<u8>,
    /// `true` when the section opens a section break with its own theme.
    pub has_theme_override: bool,
}

/// Output from listing sections.
#[derive(Debug, Serialize)]
pub struct DocxListSectionsOutput {
    /// Document ID.
    pub document_id: String,
    /// List of section summaries.
    pub sections: Vec<SectionSummary>,
    /// Total number of sections.
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

fn summarize_section(section: &DraftSection, include_content: bool) -> SectionSummary {
    let (content_type, preview, heading_level) = match &section.content {
        DocxContent::Heading { level, text, .. } => (
            "Heading".to_string(),
            truncate_preview(&text.as_plain_string(), include_content),
            Some(*level),
        ),
        DocxContent::Paragraph { text, .. } => (
            "Paragraph".to_string(),
            truncate_preview(&text.as_plain_string(), include_content),
            None,
        ),
        DocxContent::BulletList { items } => (
            "BulletList".to_string(),
            list_preview(items, include_content),
            None,
        ),
        DocxContent::NumberedList { items, .. } => (
            "NumberedList".to_string(),
            list_preview(items, include_content),
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
        has_theme_override: section.theme_override.is_some(),
    }
}

fn list_preview(items: &[ListItem], include_content: bool) -> String {
    if include_content {
        format!("{} items", items.len())
    } else {
        items
            .first()
            .map(|item| truncate_preview(&item.text.as_plain_string(), false))
            .unwrap_or_default()
    }
}

fn truncate_preview(text: &str, full: bool) -> String {
    if full || text.chars().count() <= 100 {
        text.to_string()
    } else {
        truncate_text(text, 97)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::infrastructure::tools::documents::shared::ParagraphText;

    fn heading_section(id: &str, level: u8, text: &str) -> DraftSection {
        DraftSection {
            id: id.to_string(),
            index: 0,
            content: DocxContent::Heading {
                level,
                text: ParagraphText::Plain(text.to_string()),
                alignment: None,
                page_break_before: false,
            },
            theme_override: None,
        }
    }

    #[test]
    fn summarise_heading_short() {
        let section = heading_section("sec_1", 2, "Introduction");
        let summary = summarize_section(&section, false);
        assert_eq!(summary.content_type, "Heading");
        assert_eq!(summary.preview, "Introduction");
        assert_eq!(summary.heading_level, Some(2));
        assert!(!summary.has_theme_override);
    }

    #[test]
    fn summarise_paragraph_truncates() {
        let long = "a".repeat(200);
        let section = DraftSection {
            id: "s".into(),
            index: 1,
            content: DocxContent::Paragraph {
                text: ParagraphText::Plain(long),
                alignment: None,
                role: None,
            },
            theme_override: None,
        };
        let summary = summarize_section(&section, false);
        assert!(summary.preview.chars().count() <= 100);
        assert!(summary.preview.ends_with("..."));
    }

    #[test]
    fn summarise_bullet_list_picks_first_item() {
        let section = DraftSection {
            id: "s".into(),
            index: 0,
            content: DocxContent::BulletList {
                items: vec![
                    ListItem {
                        text: ParagraphText::Plain("alpha".into()),
                        level: None,
                    },
                    ListItem {
                        text: ParagraphText::Plain("beta".into()),
                        level: None,
                    },
                ],
            },
            theme_override: None,
        };
        let summary = summarize_section(&section, false);
        assert_eq!(summary.preview, "alpha");
    }

    #[test]
    fn has_theme_override_reflects_section_state() {
        use crate::core::documents::docx::theme_patch::DocxThemePatch;
        let mut section = heading_section("s", 1, "x");
        section.theme_override = Some(DocxThemePatch::default());
        assert!(summarize_section(&section, false).has_theme_override);
    }
}
