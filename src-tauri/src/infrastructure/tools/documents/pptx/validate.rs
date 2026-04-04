//! PPTX validation utilities.

use crate::infrastructure::tools::documents::shared::draft_store::{
    PptxDraft, SlideLayout,
};
use crate::infrastructure::tools::documents::shared::validation::{
    ValidationCheck, ValidationLevel, ValidationReport,
};
use std::path::Path;

/// PPTX presentation validator.
pub struct PptxValidator;

impl PptxValidator {
    /// Validate a PPTX draft before finalization.
    #[must_use] 
    pub fn validate_draft(draft: &PptxDraft) -> ValidationReport {
        let mut report = ValidationReport::new();

        // Check 1: Has slides
        Self::validate_has_slides(&mut report, draft);

        // Check 2: Title slide exists
        Self::validate_title_slide(&mut report, draft);

        // Check 3: No empty slides
        Self::validate_no_empty_slides(&mut report, draft);

        // Check 4: Slide count
        Self::validate_slide_count(&mut report, draft);

        // Check 5: Consistent layout usage
        Self::validate_layout_consistency(&mut report, draft);

        // Check 6: Estimated duration
        Self::validate_duration(&mut report, draft);

        report
    }

    /// Validate a finalized PPTX file.
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
            passed: size > 0,
            level: ValidationLevel::Error,
            message: format!("File size: {size} bytes"),
            suggestion: if size == 0 {
                Some("File is empty and cannot be a valid presentation".to_string())
            } else {
                None
            },
        });

        // Check 3: Magic bytes
        let content = match std::fs::read(path) {
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
                "Valid ZIP signature (PPTX format)".to_string()
            } else {
                "Invalid file signature".to_string()
            },
            suggestion: Some("File may be corrupted or not a PPTX presentation".to_string()),
        });

        // Check 4: ZIP structure
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

    /// Validate that required slide types exist.
    #[must_use] 
    pub fn validate_required_slides(
        draft: &PptxDraft,
        required_slides: &[SlideLayout],
    ) -> ValidationReport {
        let mut report = ValidationReport::new();

        for layout in required_slides {
            let found = draft.slides.iter().any(|s| s.slide.layout == *layout);

            report.add_check(ValidationCheck {
                name: format!("Required Slide: {layout:?}"),
                passed: found,
                level: ValidationLevel::Warning,
                message: if found {
                    format!("Found required slide type: {layout:?}")
                } else {
                    format!("Missing recommended slide type: {layout:?}")
                },
                suggestion: if found {
                    None
                } else {
                    Some(format!("Add a slide with layout '{layout:?}'"))
                },
            });
        }

        report
    }

    /// Validate maximum slide count.
    #[must_use] 
    pub fn validate_max_slides(draft: &PptxDraft, max_slides: usize) -> ValidationReport {
        let mut report = ValidationReport::new();

        let slide_count = draft.slides.len();

        report.add_check(ValidationCheck {
            name: "Slide Limit".to_string(),
            passed: slide_count <= max_slides,
            level: ValidationLevel::Warning,
            message: format!(
                "Presentation has {slide_count} slides (max recommended: {max_slides})"
            ),
            suggestion: if slide_count > max_slides {
                Some("Consider splitting into multiple presentations".to_string())
            } else {
                None
            },
        });

        report
    }

    // === Private helper methods ===

    fn validate_has_slides(report: &mut ValidationReport, draft: &PptxDraft) {
        let has_slides = !draft.slides.is_empty();
        report.add_check(ValidationCheck {
            name: "Has Slides".to_string(),
            passed: has_slides,
            level: ValidationLevel::Error,
            message: format!("Presentation has {} slide(s)", draft.slides.len()),
            suggestion: if has_slides {
                None
            } else {
                Some("Add at least one slide to the presentation".to_string())
            },
        });
    }

    fn validate_title_slide(report: &mut ValidationReport, draft: &PptxDraft) {
        let has_title_slide = draft
            .slides
            .first()
            .is_some_and(|s| s.slide.layout == SlideLayout::TitleSlide);

        report.add_check(ValidationCheck {
            name: "Has Title Slide".to_string(),
            passed: has_title_slide,
            level: ValidationLevel::Warning,
            message: if has_title_slide {
                "First slide is a title slide".to_string()
            } else {
                "First slide is not a title slide".to_string()
            },
            suggestion: if !has_title_slide && !draft.slides.is_empty() {
                Some(
                    "Make the first slide a TitleSlide layout for better presentation structure"
                        .to_string(),
                )
            } else {
                None
            },
        });
    }

    fn validate_no_empty_slides(report: &mut ValidationReport, draft: &PptxDraft) {
        let empty_count = draft
            .slides
            .iter()
            .filter(|s| s.slide.content.is_empty() && s.slide.title.trim().is_empty())
            .count();

        report.add_check(ValidationCheck {
            name: "No Empty Slides".to_string(),
            passed: empty_count == 0,
            level: ValidationLevel::Warning,
            message: if empty_count == 0 {
                "No empty slides found".to_string()
            } else {
                format!("Found {empty_count} empty slide(s)")
            },
            suggestion: if empty_count > 0 {
                Some("Add content to empty slides or remove them".to_string())
            } else {
                None
            },
        });
    }

    fn validate_slide_count(report: &mut ValidationReport, draft: &PptxDraft) {
        let count = draft.slides.len();
        report.add_check(ValidationCheck {
            name: "Slide Count".to_string(),
            passed: count > 0,
            level: ValidationLevel::Info,
            message: format!("Presentation has {count} slides"),
            suggestion: None,
        });
    }

    fn validate_layout_consistency(report: &mut ValidationReport, draft: &PptxDraft) {
        // Check that similar content types use consistent layouts
        let title_slides = draft
            .slides
            .iter()
            .filter(|s| s.slide.layout == SlideLayout::TitleSlide)
            .count();
        let blank_slides = draft
            .slides
            .iter()
            .filter(|s| s.slide.layout == SlideLayout::Blank)
            .count();

        let mut issues = Vec::new();

        if title_slides > 1 {
            issues
                .push("Multiple title slides detected (typically only one is needed)".to_string());
        }

        if blank_slides > 0 {
            issues.push(format!(
                "{blank_slides} blank slide(s) detected - consider using specific layouts"
            ));
        }

        report.add_check(ValidationCheck {
            name: "Layout Consistency".to_string(),
            passed: issues.is_empty(),
            level: ValidationLevel::Warning,
            message: if issues.is_empty() {
                "Slide layouts are appropriately used".to_string()
            } else {
                issues.join("; ")
            },
            suggestion: if issues.is_empty() {
                None
            } else {
                Some("Review slide layouts for better presentation flow".to_string())
            },
        });
    }

    fn validate_duration(report: &mut ValidationReport, draft: &PptxDraft) {
        // Estimate: 1-2 minutes per slide
        let slide_count = draft.slides.len();
        let min_minutes = slide_count;
        let max_minutes = slide_count * 2;

        report.add_check(ValidationCheck {
            name: "Estimated Duration".to_string(),
            passed: true,
            level: ValidationLevel::Info,
            message: format!(
                "Estimated duration: {min_minutes}-{max_minutes} minutes"
            ),
            suggestion: None,
        });
    }

    fn check_zip_structure(content: &[u8]) -> bool {
        // Check for end of central directory record
        let end_sig = b"PK\x05\x06";
        content.windows(4).any(|w| w == end_sig)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::tools::documents::shared::draft_store::{DraftStatus, SlideContent};

    fn create_test_draft() -> PptxDraft {
        PptxDraft {
            deck_id: "test_deck".to_string(),
            filename: "test.pptx".to_string(),
            slides: vec![DraftSlide {
                id: "slide1".to_string(),
                index: 0,
                slide: PptxSlide {
                    layout: SlideLayout::TitleSlide,
                    title: "Test Presentation".to_string(),
                    content: vec![],
                    notes: None,
                },
            }],
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_validate_draft_with_title_slide() {
        let draft = create_test_draft();
        let report = PptxValidator::validate_draft(&draft);

        assert!(report.passed);
        assert!(report.checks.iter().any(|c| c.name == "Has Title Slide"));
    }

    #[test]
    fn test_validate_draft_empty() {
        let draft = PptxDraft {
            deck_id: "test".to_string(),
            filename: "test.pptx".to_string(),
            slides: Vec::new(),
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let report = PptxValidator::validate_draft(&draft);

        assert!(!report.passed); // Should fail due to no slides
    }

    #[test]
    fn test_validate_max_slides() {
        let draft = create_test_draft();
        let report = PptxValidator::validate_max_slides(&draft, 100);

        assert!(report.passed); // Only 1 slide, well under limit
    }

    #[test]
    fn test_validate_required_slides() {
        let draft = create_test_draft();
        let report = PptxValidator::validate_required_slides(&draft, &[SlideLayout::TitleSlide]);

        assert!(report.passed); // Has title slide
    }
}
