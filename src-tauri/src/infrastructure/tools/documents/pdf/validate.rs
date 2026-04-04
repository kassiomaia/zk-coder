//! PDF validation utilities.

use crate::infrastructure::tools::documents::shared::draft_store::{PdfDraft, PdfPageContent};
use crate::infrastructure::tools::documents::shared::validation::{
    estimate_page_count, DocumentFileType, ValidationCheck, ValidationLevel, ValidationReport,
};
use std::path::Path;

/// PDF document validator.
pub struct PdfValidator;

impl PdfValidator {
    /// Validate a PDF draft before finalization.
    #[must_use] 
    pub fn validate_draft(draft: &PdfDraft) -> ValidationReport {
        let mut report = ValidationReport::new();

        // Check 1: Has pages
        Self::validate_has_pages(&mut report, draft);

        // Check 2: Page content is not empty
        Self::validate_page_content(&mut report, draft);

        // Check 3: Page count
        Self::validate_page_count(&mut report, draft);

        report
    }

    /// Validate a finalized PDF file.
    #[must_use] 
    pub fn validate_file(path: &Path) -> ValidationReport {
        let mut report = ValidationReport::new();

        // Check 1: File exists
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
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                report.add_error(
                    "File Metadata",
                    format!("Cannot read file metadata: {e}"),
                );
                return report;
            }
        };

        let size = metadata.len();
        report.add_check(ValidationCheck {
            name: "File Size".to_string(),
            passed: size > 100, // Minimum valid PDF is larger than 100 bytes
            level: ValidationLevel::Error,
            message: format!("File size: {size} bytes"),
            suggestion: if size < 100 {
                Some("File is too small to be a valid PDF".to_string())
            } else {
                None
            },
        });

        // Check 3: Read and validate content
        let content = match std::fs::read(path) {
            Ok(c) => c,
            Err(e) => {
                report.add_error("File Read", format!("Cannot read file: {e}"));
                return report;
            }
        };

        // Check 4: PDF header
        report.add_check(ValidationCheck {
            name: "PDF Header".to_string(),
            passed: content.starts_with(b"%PDF"),
            level: ValidationLevel::Error,
            message: if content.starts_with(b"%PDF") {
                "Valid PDF header found".to_string()
            } else {
                "Missing or invalid PDF header".to_string()
            },
            suggestion: Some("File does not appear to be a valid PDF document".to_string()),
        });

        // Check 5: PDF EOF marker
        report.add_check(ValidationCheck {
            name: "PDF EOF Marker".to_string(),
            passed: Self::check_eof_marker(&content),
            level: ValidationLevel::Warning,
            message: if Self::check_eof_marker(&content) {
                "PDF EOF marker found".to_string()
            } else {
                "PDF EOF marker missing or incomplete".to_string()
            },
            suggestion: Some("PDF may not display correctly in some viewers".to_string()),
        });

        // Check 6: Check for encryption
        report.add_check(ValidationCheck {
            name: "Encryption Check".to_string(),
            passed: Self::check_encryption(&content),
            level: ValidationLevel::Warning,
            message: if Self::check_encryption(&content) {
                "PDF is not encrypted".to_string()
            } else {
                "PDF is encrypted - may have limited accessibility".to_string()
            },
            suggestion: Some("Encrypted PDFs may not allow text extraction or editing".to_string()),
        });

        // Check 7: Estimated page count (based on size)
        if let Ok(pages) = estimate_page_count(DocumentFileType::Pdf, size) {
            report.add_check(ValidationCheck {
                name: "Page Count Estimate".to_string(),
                passed: true,
                level: ValidationLevel::Info,
                message: format!("Estimated {pages} pages (based on file size)"),
                suggestion: None,
            });
        }

        report
    }

    /// Validate maximum page count.
    #[must_use] 
    pub fn validate_max_pages(draft: &PdfDraft, max_pages: usize) -> ValidationReport {
        let mut report = ValidationReport::new();

        let page_count = draft.pages.len();

        report.add_check(ValidationCheck {
            name: "Page Limit".to_string(),
            passed: page_count <= max_pages,
            level: ValidationLevel::Error,
            message: format!("Document has {page_count} pages (max: {max_pages})"),
            suggestion: if page_count > max_pages {
                Some(format!(
                    "Reduce content to fit within {max_pages} page limit"
                ))
            } else {
                None
            },
        });

        report
    }

    /// Validate that PDF contains extractable text (not image-only).
    #[must_use] 
    pub fn validate_extractable_text(draft: &PdfDraft) -> ValidationReport {
        let mut report = ValidationReport::new();

        // Check if any page has text content
        let has_text = draft.pages.iter().any(|p| match &p.content {
            PdfPageContent::Text { content } => !content.trim().is_empty(),
            PdfPageContent::Markdown { content } => !content.trim().is_empty(),
            PdfPageContent::Html { content, .. } => !content.trim().is_empty(),
            PdfPageContent::Image { .. } => false, // Images don't have extractable text
        });

        report.add_check(ValidationCheck {
            name: "Extractable Text".to_string(),
            passed: has_text,
            level: ValidationLevel::Warning,
            message: if has_text {
                "Document contains extractable text content".to_string()
            } else {
                "Document may be image-only (no extractable text)".to_string()
            },
            suggestion: if has_text {
                None
            } else {
                Some("Image-only PDFs cannot be searched or have text extracted".to_string())
            },
        });

        report
    }

    // === Private helper methods ===

    fn validate_has_pages(report: &mut ValidationReport, draft: &PdfDraft) {
        let has_pages = !draft.pages.is_empty();
        report.add_check(ValidationCheck {
            name: "Has Pages".to_string(),
            passed: has_pages,
            level: ValidationLevel::Error,
            message: format!("Document has {} page(s)", draft.pages.len()),
            suggestion: if has_pages {
                None
            } else {
                Some("Add at least one page to the document".to_string())
            },
        });
    }

    fn validate_page_content(report: &mut ValidationReport, draft: &PdfDraft) {
        let empty_pages = draft
            .pages
            .iter()
            .filter(|p| match &p.content {
                PdfPageContent::Text { content } => content.trim().is_empty(),
                PdfPageContent::Markdown { content } => content.trim().is_empty(),
                PdfPageContent::Html { content, .. } => content.trim().is_empty(),
                PdfPageContent::Image { .. } => false, // Images count as content
            })
            .count();

        report.add_check(ValidationCheck {
            name: "Page Content".to_string(),
            passed: empty_pages == 0 || empty_pages < draft.pages.len(),
            level: ValidationLevel::Warning,
            message: if empty_pages == 0 {
                "All pages have content".to_string()
            } else {
                format!("{empty_pages} page(s) are empty")
            },
            suggestion: if empty_pages > 0 {
                Some("Consider removing empty pages or adding content".to_string())
            } else {
                None
            },
        });
    }

    fn validate_page_count(report: &mut ValidationReport, draft: &PdfDraft) {
        let count = draft.pages.len();
        report.add_check(ValidationCheck {
            name: "Page Count".to_string(),
            passed: count > 0,
            level: ValidationLevel::Info,
            message: format!("Document has {count} pages"),
            suggestion: None,
        });
    }

    /// Check if PDF has valid EOF marker.
    fn check_eof_marker(content: &[u8]) -> bool {
        // PDF should end with %%EOF
        let eof_marker = b"%%EOF";
        content.len() >= 5 && content.ends_with(eof_marker)
    }

    /// Check if PDF is encrypted.
    fn check_encryption(content: &[u8]) -> bool {
        // If /Encrypt is not present, PDF is not encrypted
        let content_str = String::from_utf8_lossy(content);
        !content_str.contains("/Encrypt")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_draft() -> PdfDraft {
        PdfDraft {
            pdf_id: "test_pdf".to_string(),
            filename: "test.pdf".to_string(),
            pages: vec![
                crate::infrastructure::tools::documents::shared::draft_store::DraftPage {
                    id: "page1".to_string(),
                    index: 0,
                    content: PdfPageContent::Text {
                        content: "Test content".to_string(),
                    },
                },
            ],
            page_size:
                crate::infrastructure::tools::documents::shared::draft_store::PdfPageSize::A4,
            status:
                crate::infrastructure::tools::documents::shared::draft_store::DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_validate_draft_with_content() {
        let draft = create_test_draft();
        let report = PdfValidator::validate_draft(&draft);

        assert!(report.passed);
        assert!(report.checks.iter().any(|c| c.name == "Has Pages"));
    }

    #[test]
    fn test_validate_draft_empty() {
        let draft = PdfDraft {
            pdf_id: "test".to_string(),
            filename: "test.pdf".to_string(),
            pages: Vec::new(),
            page_size:
                crate::infrastructure::tools::documents::shared::draft_store::PdfPageSize::A4,
            status:
                crate::infrastructure::tools::documents::shared::draft_store::DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let report = PdfValidator::validate_draft(&draft);

        assert!(!report.passed); // Should fail due to no pages
    }

    #[test]
    fn test_validate_max_pages() {
        let draft = create_test_draft();
        let report = PdfValidator::validate_max_pages(&draft, 100);

        assert!(report.passed); // Only 1 page, well under limit
    }

    #[test]
    fn test_check_eof_marker() {
        let valid_pdf = b"%PDF-1.4\ncontent\n%%EOF";
        assert!(PdfValidator::check_eof_marker(valid_pdf));

        let invalid_pdf = b"%PDF-1.4\ncontent";
        assert!(!PdfValidator::check_eof_marker(invalid_pdf));
    }

    #[test]
    fn test_check_encryption() {
        let not_encrypted = b"%PDF-1.4\ncontent";
        assert!(PdfValidator::check_encryption(not_encrypted));

        let encrypted = b"%PDF-1.4\n/Encrypt 5 0 R\ncontent";
        assert!(!PdfValidator::check_encryption(encrypted));
    }

    #[test]
    fn test_validate_extractable_text() {
        let draft = create_test_draft();
        let report = PdfValidator::validate_extractable_text(&draft);

        assert!(report.passed); // Has text content
    }
}
