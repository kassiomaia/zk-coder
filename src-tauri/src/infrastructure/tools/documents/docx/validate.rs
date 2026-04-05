//! DOCX validation utilities.

use crate::infrastructure::tools::documents::shared::draft_store::{
    DocxContent, DocxDraft, DraftSection,
};
use crate::infrastructure::tools::documents::shared::validation::{
    estimate_page_count, DocumentFileType, ValidationCheck, ValidationLevel, ValidationReport,
};
use std::fs;
use std::path::Path;

/// DOCX document validator.
pub struct DocxValidator;

impl DocxValidator {
    /// Validate a DOCX draft before finalization.
    #[must_use]
    pub fn validate_draft(draft: &DocxDraft) -> ValidationReport {
        let mut report = ValidationReport::new();

        // Check 1: Draft has sections
        Self::validate_has_sections(&mut report, draft);

        // Check 2: Title exists (first heading)
        Self::validate_title(&mut report, draft);

        // Check 3: Heading hierarchy is valid
        Self::validate_heading_hierarchy(&mut report, draft);

        // Check 4: No empty paragraphs (warning)
        Self::validate_no_empty_paragraphs(&mut report, draft);

        // Check 5: Section count
        Self::validate_section_count(&mut report, draft);

        report
    }

    /// Validate a finalized DOCX file.
    #[must_use]
    pub fn validate_file(path: &Path) -> ValidationReport {
        let mut report = ValidationReport::new();

        // Check 1: File exists and is readable
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

        // Check 2: File size
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

        // Check 3: Magic bytes
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

        // Check 4: Estimated page count
        if let Ok(pages) = estimate_page_count(DocumentFileType::Docx, size) {
            report.add_check(ValidationCheck {
                name: "Page Count Estimate".to_string(),
                passed: true,
                level: ValidationLevel::Info,
                message: format!("Estimated {pages} pages"),
                suggestion: None,
            });
        }

        // Check 5: ZIP structure integrity
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

    /// Validate a draft against required sections.
    #[must_use]
    pub fn validate_required_sections(draft: &DocxDraft, required: &[&str]) -> ValidationReport {
        let mut report = ValidationReport::new();

        for section_title in required {
            let found = draft.sections.iter().any(|s| match &s.content {
                DocxContent::Heading { text, .. } => text.contains(section_title),
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

    /// Validate maximum page count.
    #[must_use]
    pub fn validate_max_pages(draft: &DocxDraft, max_pages: usize) -> ValidationReport {
        let mut report = ValidationReport::new();

        // Estimate based on section count
        let estimated_pages = Self::estimate_pages_from_sections(&draft.sections);

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

    // === Private helper methods ===

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
                // Check for skipped heading levels (e.g., H1 -> H3)
                if prev_level > 0 && *level > prev_level + 1 {
                    issues.push(format!(
                        "Skipped heading level: H{} -> H{} (should be H{})",
                        prev_level,
                        level,
                        prev_level + 1
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
                DocxContent::Paragraph { text, .. } => text.trim().is_empty(),
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
        // Check for end of central directory record
        let end_sig = b"PK\x05\x06";
        content.windows(4).any(|w| w == end_sig)
    }

    const fn estimate_pages_from_sections(sections: &[DraftSection]) -> usize {
        // Rough estimate: 15 sections per page on average
        (sections.len() / 15) + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::tools::documents::shared::draft_store::DraftStatus;
    use chrono::Utc;

    fn create_test_draft() -> DocxDraft {
        DocxDraft {
            document_id: "test_doc".to_string(),
            filename: "test.docx".to_string(),
            sections: vec![
                DraftSection {
                    id: "sec1".to_string(),
                    index: 0,
                    content: DocxContent::Heading {
                        level: 1,
                        text: "Test Document".to_string(),
                        page_break_before: false,
                    },
                },
                DraftSection {
                    id: "sec2".to_string(),
                    index: 1,
                    content: DocxContent::Paragraph {
                        text: "Test content".to_string(),
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
    fn test_validate_draft_with_content() {
        let draft = create_test_draft();
        let report = DocxValidator::validate_draft(&draft);

        assert!(report.passed);
        assert!(report.checks.iter().any(|c| c.name == "Has Content"));
    }

    #[test]
    fn test_validate_draft_empty() {
        let draft = DocxDraft {
            document_id: "test".to_string(),
            filename: "test.docx".to_string(),
            sections: Vec::new(),
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let report = DocxValidator::validate_draft(&draft);

        assert!(!report.passed); // Should fail due to no content
    }

    #[test]
    fn test_validate_required_sections() {
        let draft = create_test_draft();
        let report = DocxValidator::validate_required_sections(&draft, &["Test Document"]);

        assert!(report.passed);
    }

    #[test]
    fn test_validate_required_sections_missing() {
        let draft = create_test_draft();
        let report = DocxValidator::validate_required_sections(&draft, &["Missing Section"]);

        assert!(!report.passed);
    }

    #[test]
    fn test_validate_max_pages() {
        let draft = create_test_draft();
        let report = DocxValidator::validate_max_pages(&draft, 100);

        assert!(report.passed); // Small document, well under limit
    }

    #[test]
    fn test_estimate_pages_from_sections() {
        let sections_count = 30;
        let estimated = DocxValidator::estimate_pages_from_sections(
            &(0..sections_count)
                .map(|i| DraftSection {
                    id: format!("sec{}", i),
                    index: i,
                    content: DocxContent::Paragraph {
                        text: "Test".to_string(),
                        style: None,
                    },
                })
                .collect::<Vec<_>>(),
        );

        assert_eq!(estimated, 3); // 30 / 15 + 1
    }
}
