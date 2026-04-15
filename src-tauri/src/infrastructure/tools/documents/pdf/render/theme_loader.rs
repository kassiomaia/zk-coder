//! Load a [`Theme`] from TOML.
//!
//! The bundled `resources/pdf/themes/report.toml` is baked into the
//! binary via `include_str!`. Callers can also pass arbitrary TOML to
//! [`parse_theme`] — useful for tests and, later, for user-supplied
//! theme files.

use thiserror::Error;

use crate::core::documents::pdf::theme::Theme;

/// Bundled default report theme TOML source.
const REPORT_THEME_TOML: &str =
    include_str!("../../../../../../resources/pdf/themes/report.toml");

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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

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
