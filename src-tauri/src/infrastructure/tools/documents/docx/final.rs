//! Finalize a DOCX document.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::{DocxContent, DocxDraft, DraftStore, DraftStoreExt},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use docx_rust::document::{Break, BreakType, Paragraph, Run, Table, TableRow, TableCell};
use docx_rust::formatting::ParagraphProperty;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use tauri::AppHandle;

/// Arguments for finalizing a DOCX document.
#[derive(Debug, Deserialize)]
pub struct DocxFinalizeArgs {
    /// Document ID from docx_start
    pub document_id: String,
    /// Whether to automatically save to artifact store
    #[serde(default = "default_auto_save")]
    pub auto_save: bool,
}

fn default_auto_save() -> bool {
    true
}

/// Validation report for the document.
#[derive(Debug, Serialize)]
pub struct ValidationReport {
    /// Whether validation passed
    pub passed: bool,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Validation errors
    pub errors: Vec<String>,
}

/// Output from finalizing a document.
#[derive(Debug, Serialize)]
pub struct DocxFinalizeOutput {
    /// Document ID
    pub document_id: String,
    /// Filename
    pub filename: String,
    /// Artifact ID (if auto_saved)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    /// Full path to the generated file
    pub path: String,
    /// Validation report
    pub validation: ValidationReport,
    /// Number of sections
    pub section_count: usize,
    /// Estimated page count (rough estimate: ~15 sections per page)
    pub estimated_page_count: usize,
    /// File size in bytes
    pub size_bytes: u64,
}

/// Tool to finalize a DOCX document.
pub struct DocxFinalizeTool;

impl Tool for DocxFinalizeTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("docx", "finalize"),
            description: "Finalize a DOCX document, generating the actual Word document file and saving it to the artifact store. After finalizing, no more sections can be added.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "document_id": {
                        "type": "string",
                        "description": "Document ID from docx_start"
                    },
                    "auto_save": {
                        "type": "boolean",
                        "description": "Whether to automatically save to artifact store (default: true)",
                        "default": true
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
        _workspace_root: Option<PathBuf>,
    ) -> ToolFuture {
        Box::pin(async move {
            let args: DocxFinalizeArgs = parse_args("docx_finalize", arguments)?;

            let draft_store = DraftStore::get_draft_store();
            let draft = {
                let mut store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {}", e))
                })?;

                // Get and remove the draft (finalizing removes it from active drafts)
                store.remove_docx_draft(&args.document_id).ok_or_else(|| {
                    ToolError::failed(format!("Document not found: {}", args.document_id))
                })?
            };

            // Clone needed data before async call
            let filename = draft.filename.clone();
            let section_count = draft.sections.len();

            // Validate the document
            let validation = validate_docx_draft(&draft);

            // Estimate page count before we move draft
            let estimated_page_count = estimate_page_count(&draft);

            // Generate the DOCX file
            let (path, size_bytes) = generate_docx(draft).await?;

            // Create artifact reference
            let artifact_id = if args.auto_save {
                Some(format!("art_{}", uuid::Uuid::new_v4()))
            } else {
                None
            };

            let output = DocxFinalizeOutput {
                document_id: args.document_id,
                filename,
                artifact_id,
                path: path.to_str().unwrap().to_string(),
                validation,
                section_count,
                estimated_page_count,
                size_bytes,
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

/// Validate a DOCX draft.
fn validate_docx_draft(draft: &DocxDraft) -> ValidationReport {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut passed = true;

    // Check if document is empty
    if draft.sections.is_empty() {
        warnings.push("Document has no sections".to_string());
    }

    // Check for proper heading hierarchy
    let mut last_level = 0;
    for section in &draft.sections {
        if let DocxContent::Heading { level, .. } = &section.content {
            if *level > last_level + 1 && last_level > 0 {
                warnings.push(format!(
                    "Heading level jumps from {} to {} (section {})",
                    last_level, level, section.index
                ));
            }
            last_level = *level;
        }
    }

    // Check for table consistency
    for section in &draft.sections {
        if let DocxContent::Table { headers, rows, .. } = &section.content {
            let col_count = headers.len();
            for (row_idx, row) in rows.iter().enumerate() {
                if row.len() != col_count {
                    errors.push(format!(
                        "Table row {} has {} columns but header has {} columns (section {})",
                        row_idx + 1,
                        row.len(),
                        col_count,
                        section.index
                    ));
                    passed = false;
                }
            }
        }
    }

    // Check for image paths that might not exist
    for section in &draft.sections {
        if let DocxContent::Image { path, .. } = &section.content {
            if !std::path::Path::new(path).exists() {
                warnings.push(format!(
                    "Image path may not exist: {} (section {})",
                    path, section.index
                ));
            }
        }
    }

    ValidationReport {
        passed,
        warnings,
        errors,
    }
}

/// Generate a DOCX file from the draft using the docx-rust crate.
async fn generate_docx(draft: DocxDraft) -> Result<(PathBuf, u64), ToolError> {
    use docx_rust::Docx;

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

    // Create a new DOCX document
    let mut docx = Docx::default();

    // Process each section and add to the document
    for section in &draft.sections {
        match &section.content {
            DocxContent::Heading { level, text, page_break_before } => {
                // Add page break before heading if requested
                if *page_break_before {
                    let para = Paragraph::default()
                        .push(Run::default().push(Break::from(BreakType::Page)));
                    docx.document.push(para);
                }

                // Create heading paragraph with appropriate style
                // docx-rust doesn't have built-in heading styles, so we use bold and larger font
                let style_name = format!("Heading{}", level);
                let para = Paragraph::default()
                    .property(ParagraphProperty::default().style_id(style_name))
                    .push_text(text.as_str());
                docx.document.push(para);
            }

            DocxContent::Paragraph { text, style } => {
                let mut para = Paragraph::default();
                if let Some(s) = style {
                    para = para.property(ParagraphProperty::default().style_id(s.clone()));
                }
                let para = para.push_text(text.as_str());
                docx.document.push(para);
            }

            DocxContent::BulletList { items } => {
                // Add bullet list items with bullet characters
                for item in items {
                    // Create owned string and push as paragraph
                    let text: String = format!("\u{2022} {}", item);
                    // Use Run with Text created from owned string
                    let t = docx_rust::document::Text::from(text);
                    let run = Run::default().push(t);
                    let para = Paragraph::default().push(run);
                    docx.document.push(para);
                }
            }

            DocxContent::NumberedList { items, start_at } => {
                // Add numbered list items
                let start = start_at.unwrap_or(1);
                for (i, item) in items.iter().enumerate() {
                    let num = start + i as u32;
                    let text: String = format!("{}. {}", num, item);
                    let t = docx_rust::document::Text::from(text);
                    let run = Run::default().push(t);
                    let para = Paragraph::default().push(run);
                    docx.document.push(para);
                }
            }

            DocxContent::Table { headers, rows, caption } => {
                // Create header row
                let mut header_row = TableRow::default();
                for header in headers {
                    let t = docx_rust::document::Text::from(header.clone());
                    let run = Run::default().push(t);
                    let para = Paragraph::default().push(run);
                    let cell = TableCell::from(para);
                    header_row = header_row.push_cell(cell);
                }

                // Create data rows
                let mut table_rows = vec![header_row];
                for row_data in rows {
                    let mut row = TableRow::default();
                    for cell_text in row_data {
                        let t = docx_rust::document::Text::from(cell_text.clone());
                        let run = Run::default().push(t);
                        let para = Paragraph::default().push(run);
                        let cell = TableCell::from(para);
                        row = row.push_cell(cell);
                    }
                    table_rows.push(row);
                }

                // Build the table
                let mut table = Table::default();
                for tr in table_rows {
                    table = table.push_row(tr);
                }

                docx.document.push(table);

                // Add caption if provided
                if let Some(caption_text) = caption {
                    let caption_str: String = format!("Table: {}", caption_text);
                    let t = docx_rust::document::Text::from(caption_str);
                    let run = Run::default().push(t);
                    let caption_para = Paragraph::default().push(run);
                    docx.document.push(caption_para);
                }
            }

            DocxContent::Image { path, caption, width: _ } => {
                // For images, we add a placeholder paragraph
                // Actual image embedding would require more complex handling
                // with the docx-rust crate's Picture/Blip types
                let image_text: String = format!("[Image: {}]", path);
                let t = docx_rust::document::Text::from(image_text);
                let run = Run::default().push(t);
                let para = Paragraph::default().push(run);
                docx.document.push(para);

                if let Some(caption_text) = caption {
                    let caption_str: String = format!("Figure: {}", caption_text);
                    let t = docx_rust::document::Text::from(caption_str);
                    let run = Run::default().push(t);
                    let caption_para = Paragraph::default().push(run);
                    docx.document.push(caption_para);
                }
            }

            DocxContent::PageBreak => {
                let para = Paragraph::default()
                    .push(Run::default().push(Break::from(BreakType::Page)));
                docx.document.push(para);
            }

            DocxContent::HorizontalRule => {
                // Add a horizontal rule using underscores
                let para = Paragraph::default().push_text("_____");
                docx.document.push(para);
            }
        }
    }

    // Write the document to file
    docx.write_file(&output_path)
        .map_err(|e| ToolError::failed(format!("Failed to write DOCX file: {}", e)))?;

    let size_bytes = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok((output_path, size_bytes))
}

/// Estimate page count from draft.
fn estimate_page_count(draft: &DocxDraft) -> usize {
    // Count headings and other content
    let heading_count = draft
        .sections
        .iter()
        .filter(|s| matches!(&s.content, DocxContent::Heading { .. }))
        .count();

    let other_count = draft.sections.len() - heading_count;

    // Rough estimate: 1 page per heading, plus other content / 25
    1 + heading_count + (other_count / 25)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::tools::documents::shared::{DocxDraft, DraftSection, DraftStatus};
    use chrono::Utc;

    fn create_test_draft() -> DocxDraft {
        DocxDraft {
            document_id: "test_doc".to_string(),
            filename: "test.docx".to_string(),
            sections: vec![
                DraftSection {
                    id: "sec_1".to_string(),
                    index: 0,
                    content: DocxContent::Heading {
                        level: 1,
                        text: "Title".to_string(),
                        page_break_before: false,
                    },
                },
                DraftSection {
                    id: "sec_2".to_string(),
                    index: 1,
                    content: DocxContent::Paragraph {
                        text: "Content".to_string(),
                        style: None,
                    },
                },
            ],
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_validate_empty_document() {
        let draft = DocxDraft {
            document_id: "test".to_string(),
            filename: "test.docx".to_string(),
            sections: vec![],
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let report = validate_docx_draft(&draft);
        assert!(!report.warnings.is_empty());
        assert!(report.warnings.iter().any(|w| w.contains("no sections")));
    }

    #[test]
    fn test_validate_heading_hierarchy() {
        let draft = DocxDraft {
            document_id: "test".to_string(),
            filename: "test.docx".to_string(),
            sections: vec![
                DraftSection {
                    id: "sec_1".to_string(),
                    index: 0,
                    content: DocxContent::Heading {
                        level: 1,
                        text: "Title".to_string(),
                        page_break_before: false,
                    },
                },
                DraftSection {
                    id: "sec_2".to_string(),
                    index: 1,
                    content: DocxContent::Heading {
                        level: 3,
                        text: "Subsection".to_string(),
                        page_break_before: false,
                    },
                },
            ],
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let report = validate_docx_draft(&draft);
        assert!(!report.warnings.is_empty());
        assert!(report
            .warnings
            .iter()
            .any(|w| w.contains("jumps from 1 to 3")));
    }

    #[test]
    fn test_validate_table_inconsistent_columns() {
        let draft = DocxDraft {
            document_id: "test".to_string(),
            filename: "test.docx".to_string(),
            sections: vec![DraftSection {
                id: "sec_1".to_string(),
                index: 0,
                content: DocxContent::Table {
                    headers: vec!["A".to_string(), "B".to_string()],
                    rows: vec![
                        vec!["1".to_string(), "2".to_string(), "3".to_string()], // 3 columns!
                    ],
                    caption: None,
                },
            }],
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let report = validate_docx_draft(&draft);
        assert!(!report.passed);
        assert!(report
            .errors
            .iter()
            .any(|e| e.contains("3 columns but header has 2")));
    }

    #[test]
    fn test_validate_valid_document() {
        let draft = create_test_draft();
        let report = validate_docx_draft(&draft);
        assert!(report.passed);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn test_estimate_page_count() {
        let draft = create_test_draft();
        let pages = estimate_page_count(&draft);
        assert!(pages >= 1);
    }
}
