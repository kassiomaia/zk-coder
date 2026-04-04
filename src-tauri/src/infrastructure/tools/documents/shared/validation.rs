//! Validation utilities for document artifacts.
//!
//! Provides common validation types and utilities used across
//! DOCX, PPTX, and PDF validators.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Result of a validation check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    /// Name of the check
    pub name: String,
    /// Whether the check passed
    pub passed: bool,
    /// Severity level
    pub level: ValidationLevel,
    /// Message describing the result
    pub message: String,
    /// Additional context or suggestions
    pub suggestion: Option<String>,
}

/// Severity level for validation issues.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValidationLevel {
    /// Information only, no issue
    Info,
    /// Warning that doesn't prevent use
    Warning,
    /// Error that makes the document invalid
    Error,
    /// Critical error that corrupts the document
    Critical,
}

/// Complete validation report for a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Overall validation status
    pub passed: bool,
    /// Individual validation checks
    pub checks: Vec<ValidationCheck>,
    /// Total checks performed
    pub total_checks: usize,
    /// Passed checks count
    pub passed_checks: usize,
    /// Failed checks count
    pub failed_checks: usize,
    /// Warning count
    pub warning_count: usize,
    /// Error count
    pub error_count: usize,
}

impl ValidationReport {
    /// Create a new empty validation report.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            passed: true,
            checks: Vec::new(),
            total_checks: 0,
            passed_checks: 0,
            failed_checks: 0,
            warning_count: 0,
            error_count: 0,
        }
    }

    /// Add a validation check to the report.
    pub fn add_check(&mut self, check: ValidationCheck) {
        self.total_checks += 1;

        match check.level {
            ValidationLevel::Info => {
                self.passed_checks += 1;
            }
            ValidationLevel::Warning => {
                self.warning_count += 1;
            }
            ValidationLevel::Error | ValidationLevel::Critical => {
                if check.passed {
                    self.passed_checks += 1;
                } else {
                    self.error_count += 1;
                    self.failed_checks += 1;
                    self.passed = false;
                }
            }
        }

        self.checks.push(check);
    }

    /// Add a passing check.
    pub fn add_pass(&mut self, name: impl Into<String>, message: impl Into<String>) {
        self.add_check(ValidationCheck {
            name: name.into(),
            passed: true,
            level: ValidationLevel::Info,
            message: message.into(),
            suggestion: None,
        });
    }

    /// Add a warning check.
    pub fn add_warning(&mut self, name: impl Into<String>, message: impl Into<String>) {
        self.add_check(ValidationCheck {
            name: name.into(),
            passed: true,
            level: ValidationLevel::Warning,
            message: message.into(),
            suggestion: None,
        });
    }

    /// Add an error check.
    pub fn add_error(&mut self, name: impl Into<String>, message: impl Into<String>) {
        self.add_check(ValidationCheck {
            name: name.into(),
            passed: false,
            level: ValidationLevel::Error,
            message: message.into(),
            suggestion: None,
        });
    }

    /// Get all errors from the report.
    #[must_use]
    pub fn errors(&self) -> Vec<&ValidationCheck> {
        self.checks
            .iter()
            .filter(|c| c.level == ValidationLevel::Error || c.level == ValidationLevel::Critical)
            .collect()
    }

    /// Get all warnings from the report.
    #[must_use]
    pub fn warnings(&self) -> Vec<&ValidationCheck> {
        self.checks
            .iter()
            .filter(|c| c.level == ValidationLevel::Warning)
            .collect()
    }
}

impl Default for ValidationReport {
    fn default() -> Self {
        Self {
            passed: true, // Start as true, only set to false on errors
            checks: Vec::new(),
            total_checks: 0,
            passed_checks: 0,
            failed_checks: 0,
            warning_count: 0,
            error_count: 0,
        }
    }
}

/// Validate that a file exists and is readable.
pub fn validate_file_exists(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("File does not exist: {}", path.display()));
    }
    if !path.is_file() {
        return Err(format!("Path is not a file: {}", path.display()));
    }
    Ok(())
}

/// Validate file magic bytes for a given type.
pub fn validate_magic_bytes(content: &[u8], expected_type: DocumentFileType) -> Result<(), String> {
    if content.len() < 4 {
        return Err("File content is too short to be valid".to_string());
    }

    let valid = match expected_type {
        DocumentFileType::Docx | DocumentFileType::Pptx => {
            // ZIP signature
            &content[0..4] == b"PK\x03\x04" || &content[0..4] == b"PK\x05\x06"
        }
        DocumentFileType::Pdf => &content[0..4] == b"%PDF",
    };

    if !valid {
        return Err(format!(
            "File does not have correct magic bytes for {expected_type:?}"
        ));
    }

    Ok(())
}

/// Document file type for validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentFileType {
    /// Microsoft Word document.
    Docx,
    /// Microsoft `PowerPoint` presentation.
    Pptx,
    /// Portable Document Format (PDF) file.
    Pdf,
}

/// Validate file size against limits.
pub fn validate_file_size(size: u64, max_size: u64) -> Result<(), String> {
    if size > max_size {
        return Err(format!(
            "File size {size} bytes exceeds maximum {max_size} bytes"
        ));
    }
    Ok(())
}

/// Calculate estimated page count from content size.
///
/// This is a rough estimate based on typical content density.
pub const fn estimate_page_count(
    file_type: DocumentFileType,
    size_bytes: u64,
) -> Result<usize, String> {
    // Rough estimates: average content per page
    let bytes_per_page = match file_type {
        DocumentFileType::Docx => 25_000,  // ~25KB per page with formatting
        DocumentFileType::Pptx => 100_000, // ~100KB per slide with images
        DocumentFileType::Pdf => 50_000,   // ~50KB per page
    };

    if size_bytes == 0 {
        return Ok(0);
    }

    Ok((size_bytes / bytes_per_page + 1) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_report_new() {
        let report = ValidationReport::new();
        assert!(report.passed);
        assert_eq!(report.total_checks, 0);
    }

    #[test]
    fn test_validation_report_add_pass() {
        let mut report = ValidationReport::new();
        report.add_pass("test_check", "All good");
        assert_eq!(report.total_checks, 1);
        assert_eq!(report.passed_checks, 1);
        assert!(report.passed);
    }

    #[test]
    fn test_validation_report_add_warning() {
        let mut report = ValidationReport::new();
        report.add_warning("test_check", "Minor issue");
        assert_eq!(report.total_checks, 1);
        assert_eq!(report.warning_count, 1);
        assert!(report.passed); // Warnings don't fail the report
    }

    #[test]
    fn test_validation_report_add_error() {
        let mut report = ValidationReport::new();
        report.add_error("test_check", "Major issue");
        assert_eq!(report.total_checks, 1);
        assert_eq!(report.error_count, 1);
        assert!(!report.passed); // Errors fail the report
    }

    #[test]
    fn test_validate_magic_bytes_docx() {
        let content = b"PK\x03\x04test content";
        assert!(validate_magic_bytes(content, DocumentFileType::Docx).is_ok());
    }

    #[test]
    fn test_validate_magic_bytes_pdf() {
        let content = b"%PDF-1.4 test content";
        assert!(validate_magic_bytes(content, DocumentFileType::Pdf).is_ok());
    }

    #[test]
    fn test_validate_magic_bytes_invalid() {
        let content = b"NOT A VALID FILE";
        assert!(validate_magic_bytes(content, DocumentFileType::Pdf).is_err());
    }

    #[test]
    fn test_estimate_page_count() {
        let count = estimate_page_count(DocumentFileType::Docx, 100_000).unwrap();
        assert_eq!(count, 5); // ~100KB / 25KB per page + 1
    }

    #[test]
    fn test_validate_file_size() {
        assert!(validate_file_size(1024, 2048).is_ok());
        assert!(validate_file_size(2048, 1024).is_err());
    }

    #[test]
    fn test_validation_report_passed_initially_true() {
        let report = ValidationReport::new();
        assert!(report.passed, "New report should pass");
    }

    #[test]
    fn test_validation_report_adding_pass_keeps_passed() {
        let mut report = ValidationReport::new();
        report.add_pass("Test", "Test message");
        assert!(report.passed, "Report with passing checks should pass");
    }

    #[test]
    fn test_validation_report_adding_error_fails() {
        let mut report = ValidationReport::new();
        report.add_error("Test", "Test error");
        assert!(!report.passed, "Report with error should fail");
    }
}
