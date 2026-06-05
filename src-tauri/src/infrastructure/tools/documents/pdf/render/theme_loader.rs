//! Load a [`Theme`] from TOML.
//!
//! The bundled `resources/pdf/themes/report.toml` is baked into the
//! binary via `include_str!`. Callers can also pass arbitrary TOML to
//! [`parse_theme`] — useful for tests and, later, for user-supplied
//! theme files.

use thiserror::Error;

use crate::core::documents::pdf::theme::Theme;
use crate::core::documents::pdf::theme_patch::ThemePatch;
use crate::infrastructure::tools::documents::shared::PdfDraft;

/// Bundled default report theme TOML source.
const REPORT_THEME_TOML: &str = include_str!("../../../../../../resources/pdf/themes/report.toml");

/// Failure mode for [`parse_theme`] / [`load_report_theme`].
#[derive(Debug, Error)]
pub enum ThemeLoadError {
    /// The input could not be parsed as TOML into a [`Theme`].
    #[error("invalid theme TOML: {0}")]
    Parse(#[from] toml::de::Error),
}

/// Parses TOML text into a [`Theme`].
///
/// # Errors
/// Returns [`ThemeLoadError::Parse`] if the TOML does not match the
/// schema (missing fields, wrong types, ...).
pub fn parse_theme(toml_text: &str) -> Result<Theme, ThemeLoadError> {
    let theme: Theme = toml::from_str(toml_text)?;
    Ok(theme)
}

/// Loads the bundled report theme.
///
/// Uses the TOML embedded at build time. Falls back to
/// [`Theme::default_report`] if the bundled source ever fails to parse —
/// this should never happen in production because the sweep test in the
/// renderer keeps the two in sync, but the fallback exists so the tool
/// stays runnable even under misconfiguration.
#[must_use]
pub fn load_report_theme() -> Theme {
    parse_theme(REPORT_THEME_TOML).unwrap_or_else(|_| Theme::default_report())
}

/// Resolves the effective [`Theme`] for a PDF draft.
///
/// Starts from the bundled report theme and overlays the draft's
/// optional [`ThemePatch`]. `Some` fields in the patch replace; absent
/// fields leave the base untouched. This is the single render-time seam
/// that lets LLM-supplied styling take effect.
#[must_use]
pub fn theme_for_draft(draft: &PdfDraft) -> Theme {
    let base = load_report_theme();
    match draft.theme_patch.as_ref() {
        Some(patch) => base.with_patch(patch),
        None => base,
    }
}

/// Convenience: apply a patch to the bundled report theme directly.
///
/// Useful for tests that exercise the merge path without constructing a
/// full [`PdfDraft`].
#[must_use]
pub fn report_theme_with_patch(patch: &ThemePatch) -> Theme {
    load_report_theme().with_patch(patch)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::core::documents::pdf::theme_patch::{PalettePatch, RgbPatch, TypographyPatch};
    use crate::infrastructure::tools::documents::shared::draft_store::{DraftStatus, PdfPageSize};
    use chrono::Utc;

    fn bare_draft(patch: Option<ThemePatch>) -> PdfDraft {
        PdfDraft {
            pdf_id: "pdf_test".to_string(),
            filename: "test.pdf".to_string(),
            title: "T".to_string(),
            pages: Vec::new(),
            page_size: PdfPageSize::A4,
            theme_patch: patch,
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn theme_for_draft_without_patch_matches_bundled() {
        let theme = theme_for_draft(&bare_draft(None));
        assert_eq!(theme, load_report_theme());
    }

    #[test]
    fn theme_for_draft_applies_typography_patch() {
        let patch = ThemePatch {
            typography: Some(TypographyPatch {
                body_font_size_pt: Some(14.0),
                ..TypographyPatch::default()
            }),
            ..ThemePatch::default()
        };
        let theme = theme_for_draft(&bare_draft(Some(patch)));
        assert!((theme.typography.body_font_size_pt - 14.0).abs() < f32::EPSILON);
        // Heading size left at default (bundled).
        assert!(
            (theme.typography.heading_font_size_pt
                - load_report_theme().typography.heading_font_size_pt)
                .abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn theme_for_draft_applies_palette_patch() {
        let patch = ThemePatch {
            palette: Some(PalettePatch {
                heading: Some(RgbPatch {
                    r: Some(0.1),
                    g: Some(0.2),
                    b: Some(0.8),
                }),
                ..PalettePatch::default()
            }),
            ..ThemePatch::default()
        };
        let theme = theme_for_draft(&bare_draft(Some(patch)));
        assert!((theme.palette.heading.r - 0.1).abs() < f32::EPSILON);
        assert!((theme.palette.heading.g - 0.2).abs() < f32::EPSILON);
        assert!((theme.palette.heading.b - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn bundled_report_toml_parses() {
        let theme = parse_theme(REPORT_THEME_TOML).expect("bundled report.toml must parse");
        assert!(theme.typography.body_font_size_pt > 0.0);
        assert!(theme.typography.heading_font_size_pt > theme.typography.body_font_size_pt);
        assert!(theme.typography.line_leading > 1.0);
        assert!(theme.spacing.block_gap > 0.0);
        assert!(theme.page_defaults.margin_mm > 0.0);
    }

    #[test]
    fn bundled_matches_hardcoded_default() {
        let loaded = parse_theme(REPORT_THEME_TOML).expect("parse");
        let fallback = Theme::default_report();
        assert_eq!(
            loaded, fallback,
            "bundled report.toml must stay in sync with Theme::default_report"
        );
    }

    #[test]
    fn parse_rejects_missing_section() {
        let broken = r#"
[typography]
body_font_size_pt = 11.0
heading_font_size_pt = 18.0
line_leading = 1.3
"#;
        let err = parse_theme(broken).expect_err("missing palette must fail");
        assert!(matches!(err, ThemeLoadError::Parse(_)));
    }

    #[test]
    fn load_report_theme_never_panics() {
        let _ = load_report_theme();
    }
}
