//! DOCX validation utilities.
//!
//! Single source of truth for draft-level checks. `docx_finalize`
//! feeds the output through [`ValidationReport::into_draft_report`] to
//! build the user-facing payload and, on hard errors, returns a
//! [`ToolError::Structured`] so the LLM receives machine-actionable
//! failure details.

use crate::infrastructure::tools::documents::shared::validation::{
    estimate_page_count, DocumentFileType, ValidationCheck, ValidationLevel, ValidationReport,
};
use crate::infrastructure::tools::documents::shared::{
    DocxContent, DocxDraft, DraftSection, ParagraphText,
};
use std::fs;
use std::path::Path;

/// DOCX document validator.
pub struct DocxValidator;

impl DocxValidator {
    /// Run every draft-level check.
    #[must_use]
    pub fn validate_draft(draft: &DocxDraft) -> ValidationReport {
        let mut report = ValidationReport::new();

        Self::validate_has_sections(&mut report, draft);
        Self::validate_title(&mut report, draft);
        Self::validate_heading_hierarchy(&mut report, draft);
        Self::validate_no_empty_paragraphs(&mut report, draft);
        Self::validate_table_consistency(&mut report, draft);
        Self::validate_image_paths(&mut report, draft);
        Self::validate_section_count(&mut report, draft);

        report
    }

    /// Run checks against the generated file on disk.
    #[must_use]
    pub fn validate_file(path: &Path) -> ValidationReport {
        let mut report = ValidationReport::new();

        report.add_check(ValidationCheck {
            name: "File Exists".to_string(),
            passed: path.exists(),
            level: ValidationLevel::Error,
            message: if path.exists() {
                "File exists and is accessible".to_string()
            } else {
                format!("File not found: {}", path.display())
            },
            suggestion: None,
        });

        if !path.exists() {
            return report;
        }

        let metadata = match fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                report.add_error("File Metadata", format!("Cannot read file metadata: {e}"));
                return report;
            }
        };
        let size = metadata.len();
        report.add_check(ValidationCheck {
            name: "File Size".to_string(),
            passed: size > 0,
            level: ValidationLevel::Error,
            message: format!("File size: {size} bytes"),
            suggestion: if size == 0 {
                Some("File is empty and cannot be a valid document".to_string())
            } else {
                None
            },
        });

        let content = match fs::read(path) {
            Ok(c) => c,
            Err(e) => {
                report.add_error("File Read", format!("Cannot read file: {e}"));
                return report;
            }
        };
        report.add_check(ValidationCheck {
            name: "File Signature".to_string(),
            passed: content.len() >= 4 && &content[0..4] == b"PK\x03\x04",
            level: ValidationLevel::Error,
            message: if content.len() >= 4 && &content[0..4] == b"PK\x03\x04" {
                "Valid ZIP signature (DOCX format)".to_string()
            } else {
                "Invalid file signature".to_string()
            },
            suggestion: Some("File may be corrupted or not a DOCX document".to_string()),
        });

        if let Ok(pages) = estimate_page_count(DocumentFileType::Docx, size) {
            report.add_check(ValidationCheck {
                name: "Page Count Estimate".to_string(),
                passed: true,
                level: ValidationLevel::Info,
                message: format!("Estimated {pages} pages"),
                suggestion: None,
            });
        }

        report.add_check(ValidationCheck {
            name: "ZIP Structure".to_string(),
            passed: Self::check_zip_structure(&content),
            level: ValidationLevel::Error,
            message: if Self::check_zip_structure(&content) {
                "ZIP structure is intact".to_string()
            } else {
                "ZIP structure appears corrupted".to_string()
            },
            suggestion: None,
        });

        report
    }

    /// Validate against required section titles (matched as substrings
    /// of heading text).
    #[must_use]
    pub fn validate_required_sections(draft: &DocxDraft, required: &[&str]) -> ValidationReport {
        let mut report = ValidationReport::new();

        for section_title in required {
            let found = draft.sections.iter().any(|s| match &s.content {
                DocxContent::Heading { text, .. } => {
                    text.as_plain_string().contains(section_title)
                }
                _ => false,
            });

            report.add_check(ValidationCheck {
                name: format!("Required Section: {section_title}"),
                passed: found,
                level: ValidationLevel::Error,
                message: if found {
                    format!("Found required section: {section_title}")
                } else {
                    format!("Missing required section: {section_title}")
                },
                suggestion: if found {
                    None
                } else {
                    Some(format!(
                        "Add a heading section with title containing '{section_title}'"
                    ))
                },
            });
        }

        report
    }

    /// Estimate-based page limit check.
    #[must_use]
    pub fn validate_max_pages(draft: &DocxDraft, max_pages: usize) -> ValidationReport {
        let mut report = ValidationReport::new();
        let estimated_pages = estimate_pages_from_sections(&draft.sections);
        report.add_check(ValidationCheck {
            name: "Page Limit".to_string(),
            passed: estimated_pages <= max_pages,
            level: ValidationLevel::Error,
            message: format!("Estimated {estimated_pages} pages (max: {max_pages})"),
            suggestion: if estimated_pages > max_pages {
                Some(format!(
                    "Reduce content or split into multiple documents (max {max_pages} pages)"
                ))
            } else {
                None
            },
        });
        report
    }

    fn validate_has_sections(report: &mut ValidationReport, draft: &DocxDraft) {
        let has_sections = !draft.sections.is_empty();
        report.add_check(ValidationCheck {
            name: "Has Content".to_string(),
            passed: has_sections,
            level: ValidationLevel::Error,
            message: format!("Document has {} sections", draft.sections.len()),
            suggestion: if has_sections {
                None
            } else {
                Some("Add at least one section to the document".to_string())
            },
        });
    }

    fn validate_title(report: &mut ValidationReport, draft: &DocxDraft) {
        let has_title = draft.sections.iter().any(|s| match &s.content {
            DocxContent::Heading { level, .. } => *level == 1,
            _ => false,
        });
        report.add_check(ValidationCheck {
            name: "Has Title".to_string(),
            passed: has_title,
            level: ValidationLevel::Warning,
            message: if has_title {
                "Document has a title (H1 heading)".to_string()
            } else {
                "Document is missing a title (H1 heading)".to_string()
            },
            suggestion: if has_title {
                None
            } else {
                Some("Add a level 1 heading at the beginning of the document".to_string())
            },
        });
    }

    fn validate_heading_hierarchy(report: &mut ValidationReport, draft: &DocxDraft) {
        let mut issues = Vec::new();
        let mut prev_level = 0;
        for section in &draft.sections {
            if let DocxContent::Heading { level, .. } = &section.content {
                if prev_level > 0 && *level > prev_level + 1 {
                    issues.push(format!(
                        "Skipped heading level: H{prev} -> H{cur} (should be H{next})",
                        prev = prev_level,
                        cur = level,
                        next = prev_level + 1
                    ));
                }
                prev_level = *level;
            }
        }
        report.add_check(ValidationCheck {
            name: "Heading Hierarchy".to_string(),
            passed: issues.is_empty(),
            level: ValidationLevel::Warning,
            message: if issues.is_empty() {
                "Heading hierarchy is valid".to_string()
            } else {
                format!("Heading hierarchy issues: {}", issues.join(", "))
            },
            suggestion: if issues.is_empty() {
                None
            } else {
                Some("Use sequential heading levels (H1 -> H2 -> H3)".to_string())
            },
        });
    }

    fn validate_no_empty_paragraphs(report: &mut ValidationReport, draft: &DocxDraft) {
        let empty_count = draft
            .sections
            .iter()
            .filter(|s| match &s.content {
                DocxContent::Paragraph { text, .. } => paragraph_text_is_blank(text),
                _ => false,
            })
            .count();
        report.add_check(ValidationCheck {
            name: "No Empty Paragraphs".to_string(),
            passed: empty_count == 0,
            level: ValidationLevel::Warning,
            message: if empty_count == 0 {
                "No empty paragraphs found".to_string()
            } else {
                format!("Found {empty_count} empty paragraph(s)")
            },
            suggestion: if empty_count > 0 {
                Some("Remove empty paragraphs or add content".to_string())
            } else {
                None
            },
        });
    }

    fn validate_table_consistency(report: &mut ValidationReport, draft: &DocxDraft) {
        let mut errors = Vec::new();
        for section in &draft.sections {
            if let DocxContent::Table { headers, rows, .. } = &section.content {
                let col_count = headers.len();
                for (row_idx, row) in rows.iter().enumerate() {
                    if row.len() != col_count {
                        errors.push(format!(
                            "Section {idx}: table row {row_idx} has {rc} columns but header has {hc}",
                            idx = section.index,
                            row_idx = row_idx + 1,
                            rc = row.len(),
                            hc = col_count,
                        ));
                    }
                }
            }
        }
        report.add_check(ValidationCheck {
            name: "Table Shape".to_string(),
            passed: errors.is_empty(),
            level: ValidationLevel::Error,
            message: if errors.is_empty() {
                "All tables have consistent column counts".to_string()
            } else {
                errors.join("; ")
            },
            suggestion: if errors.is_empty() {
                None
            } else {
                Some("Each table row must match the header column count.".to_string())
            },
        });
    }

    fn validate_image_paths(report: &mut ValidationReport, draft: &DocxDraft) {
        let mut missing = Vec::new();
        for section in &draft.sections {
            if let DocxContent::Image { path, .. } = &section.content {
                if !Path::new(path).exists() {
                    missing.push(format!("section {}: {path}", section.index));
                }
            }
        }
        report.add_check(ValidationCheck {
            name: "Image Files".to_string(),
            passed: missing.is_empty(),
            level: ValidationLevel::Error,
            message: if missing.is_empty() {
                "All image files exist".to_string()
            } else {
                format!("Missing image files: {}", missing.join(", "))
            },
            suggestion: if missing.is_empty() {
                None
            } else {
                Some("Provide absolute paths to existing PNG or JPEG files.".to_string())
            },
        });
    }

    fn validate_section_count(report: &mut ValidationReport, draft: &DocxDraft) {
        let count = draft.sections.len();
        report.add_check(ValidationCheck {
            name: "Section Count".to_string(),
            passed: count > 0,
            level: ValidationLevel::Info,
            message: format!("Document has {count} sections"),
            suggestion: None,
        });
    }

    fn check_zip_structure(content: &[u8]) -> bool {
        let end_sig = b"PK\x05\x06";
        content.windows(4).any(|w| w == end_sig)
    }
}

fn paragraph_text_is_blank(text: &ParagraphText) -> bool {
    match text {
        ParagraphText::Plain(s) => s.trim().is_empty(),
        ParagraphText::Rich(runs) => runs.iter().all(|r| r.text.trim().is_empty()),
    }
}

const fn estimate_pages_from_sections(sections: &[DraftSection]) -> usize {
    (sections.len() / 15) + 1
}

/// Public helper so finalize can emit the same estimate.
#[must_use]
pub fn estimate_pages(draft: &DocxDraft) -> usize {
    (draft.sections.len() / 15) + 1
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::infrastructure::tools::documents::shared::{DraftSection, DraftStatus};
    use chrono::Utc;

    fn heading(level: u8, text: &str) -> DraftSection {
        DraftSection {
            id: "s".to_string(),
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

    fn draft(sections: Vec<DraftSection>) -> DocxDraft {
        DocxDraft {
            document_id: "test".to_string(),
            filename: "test.docx".to_string(),
            title: "Test".to_string(),
            sections,
            theme_patch: None,
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn empty_draft_fails_has_content() {
        let report = DocxValidator::validate_draft(&draft(vec![]));
        assert!(!report.passed);
        assert!(report.checks.iter().any(|c| c.name == "Has Content" && !c.passed));
    }

    #[test]
    fn skipped_heading_level_surfaces_warning() {
        let d = draft(vec![heading(1, "a"), heading(3, "b")]);
        let report = DocxValidator::validate_draft(&d);
        assert!(report.checks.iter().any(|c| c.name == "Heading Hierarchy" && !c.passed));
    }

    #[test]
    fn table_with_mismatched_row_fails() {
        let d = draft(vec![DraftSection {
            id: "s".into(),
            index: 0,
            content: DocxContent::Table {
                headers: vec![ParagraphText::Plain("A".into()), ParagraphText::Plain("B".into())],
                rows: vec![vec![ParagraphText::Plain("1".into())]],
                caption: None,
            },
            theme_override: None,
        }]);
        let report = DocxValidator::validate_draft(&d);
        assert!(report.checks.iter().any(|c| c.name == "Table Shape" && !c.passed));
    }

    #[test]
    fn missing_image_fails() {
        let d = draft(vec![DraftSection {
            id: "s".into(),
            index: 0,
            content: DocxContent::Image {
                path: "/nonexistent/path.png".into(),
                caption: None,
                width_mm: None,
            },
            theme_override: None,
        }]);
        let report = DocxValidator::validate_draft(&d);
        assert!(report.checks.iter().any(|c| c.name == "Image Files" && !c.passed));
    }
}
