//! Typed theme describing every visual knob used by the renderer.
//!
//! A `Theme` is the single source of truth for page margins, font sizes,
//! colors, and spacing. The loader in
//! `infrastructure/.../pdf/render/theme_loader.rs` materialises one from a
//! bundled TOML file. T1.4 retires the ad-hoc constants previously spread
//! through the render path.

use serde::{Deserialize, Serialize};

/// Font sizes and leading expressed in typographic units.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct TypographyScale {
    /// Body font size, in points.
    pub body_font_size_pt: f32,
    /// Heading font size, in points.
    pub heading_font_size_pt: f32,
    /// Line leading as a multiplier of the font size.
    pub line_leading: f32,
}

/// Normalised RGB triple in the `[0.0, 1.0]` unit cube.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct Rgb {
    /// Red channel.
    pub r: f32,
    /// Green channel.
    pub g: f32,
    /// Blue channel.
    pub b: f32,
}

/// Colors applied by the renderer.
///
/// The T1.4 renderer uses `text` only; additional slots are declared so
/// later milestones (M2 tables, M2 code blocks) do not change the schema.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct Palette {
    /// Body text color.
    pub text: Rgb,
    /// Heading text color.
    pub heading: Rgb,
}

/// Block-level spacing multipliers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct Spacing {
    /// Vertical gap between blocks, as a multiplier of the body leading.
    pub block_gap: f32,
}

/// Page-level defaults applied when the draft does not override them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct PageDefaults {
    /// Uniform page margin in millimetres.
    pub margin_mm: f32,
}

/// Complete theme consumed by the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct Theme {
    /// Font sizes and leading.
    pub typography: TypographyScale,
    /// Color palette.
    pub palette: Palette,
    /// Block spacing.
    pub spacing: Spacing,
    /// Page-level defaults.
    pub page_defaults: PageDefaults,
}

impl Theme {
    /// Hard-coded fallback theme used if the bundled `report.toml` cannot
    /// be loaded. Values mirror `resources/pdf/themes/report.toml`.
    #[must_use]
    pub const fn default_report() -> Self {
        Self {
            typography: TypographyScale {
                body_font_size_pt: 11.0,
                heading_font_size_pt: 18.0,
                line_leading: 1.3,
            },
            palette: Palette {
                text: Rgb {
                    r: 0.05,
                    g: 0.05,
                    b: 0.05,
                },
                heading: Rgb {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                },
            },
            spacing: Spacing { block_gap: 0.4 },
            page_defaults: PageDefaults { margin_mm: 20.0 },
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_report()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn default_report_has_positive_scale() {
        let t = Theme::default_report();
        assert!(t.typography.body_font_size_pt > 0.0);
        assert!(t.typography.heading_font_size_pt > t.typography.body_font_size_pt);
        assert!(t.typography.line_leading > 1.0);
        assert!(t.page_defaults.margin_mm > 0.0);
    }

    #[test]
    fn serde_roundtrip() {
        let t = Theme::default_report();
        let json = serde_json::to_string(&t).unwrap();
        let back: Theme = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }
}
