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
/// `text` / `heading` drive body copy. The `code_*` and `callout_*`
/// slots back the M2 code-block and callout surfaces — they live in the
/// theme so the sweep test (no hex literals in the render path) holds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct Palette {
    /// Body text color.
    pub text: Rgb,
    /// Heading text color.
    pub heading: Rgb,
    /// Background fill for code blocks.
    pub code_bg: Rgb,
    /// Foreground text color for code blocks.
    pub code_text: Rgb,
    /// Background fill for callout boxes.
    pub callout_bg: Rgb,
    /// Border / accent color for callout boxes.
    pub callout_border: Rgb,
    /// Background fill for table header rows.
    pub table_header_bg: Rgb,
    /// Table border / gridline color.
    pub table_border: Rgb,
}

/// Block-level spacing multipliers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct Spacing {
    /// Vertical gap between blocks, as a multiplier of the body leading.
    pub block_gap: f32,
    /// Horizontal indent per list-nesting level, in mm.
    pub list_indent_mm: f32,
    /// Internal padding around code blocks and callouts, in mm.
    pub block_padding_mm: f32,
    /// Internal cell padding within table cells, in mm.
    pub table_cell_padding_mm: f32,
}

/// Page-level defaults applied when the draft does not override them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct PageDefaults {
    /// Uniform page margin in millimetres.
    pub margin_mm: f32,
}

/// Header/footer templates and their styling.
///
/// Templates support the tokens `{title}`, `{page}`, `{total_pages}`,
/// and `{section}`. Unknown tokens are left literal so theme typos
/// surface in the rendered PDF instead of being silently dropped.
///
/// Either side (header or footer) can be `None` to omit that strip
/// entirely; both being `None` disables decoration outright.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct Decorations {
    /// Template painted at the top of each page (subject to
    /// `suppress_on_first_page`).
    #[serde(default)]
    pub header: Option<String>,
    /// Template painted at the bottom of each page.
    #[serde(default)]
    pub footer: Option<String>,
    /// Skip header & footer on the first page (cover convention).
    #[serde(default)]
    pub suppress_on_first_page: bool,
    /// Font size applied to header/footer text, in points.
    pub font_size_pt: f32,
    /// Distance from page edge to header/footer baseline, in mm.
    pub margin_mm: f32,
}

impl Default for Decorations {
    fn default() -> Self {
        // No header/footer by default — opt in via theme.
        Self {
            header: None,
            footer: None,
            suppress_on_first_page: true,
            font_size_pt: 9.0,
            margin_mm: 10.0,
        }
    }
}

/// Font family paths resolved relative to the bundled font directory
/// (`src-tauri/resources/pdf/fonts/`).
///
/// Every field is a file name, not a full path — the loader joins them
/// with the resource root at runtime. Fallbacks are consulted for glyphs
/// missing from the style-selected family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fonts {
    /// Regular weight used for plain body text.
    pub regular: String,
    /// Bold weight.
    pub bold: String,
    /// Italic / oblique weight.
    pub italic: String,
    /// Monospaced weight used for inline code runs.
    pub mono: String,
    /// Families consulted when a glyph is missing from the style-selected
    /// font. Walked left to right.
    #[serde(default)]
    pub fallbacks: Vec<String>,
}

/// Complete theme consumed by the renderer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq, clippy::module_name_repetitions)]
pub struct Theme {
    /// Font sizes and leading.
    pub typography: TypographyScale,
    /// Color palette.
    pub palette: Palette,
    /// Block spacing.
    pub spacing: Spacing,
    /// Page-level defaults.
    pub page_defaults: PageDefaults,
    /// Header / footer templates and styling.
    #[serde(default)]
    pub decorations: Decorations,
    /// Font families bundled with this theme.
    pub fonts: Fonts,
}

impl Theme {
    /// Hard-coded fallback theme used if the bundled `report.toml` cannot
    /// be loaded. Values mirror `resources/pdf/themes/report.toml`.
    #[must_use]
    pub fn default_report() -> Self {
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
                code_bg: Rgb {
                    r: 0.96,
                    g: 0.96,
                    b: 0.97,
                },
                code_text: Rgb {
                    r: 0.12,
                    g: 0.12,
                    b: 0.15,
                },
                callout_bg: Rgb {
                    r: 0.93,
                    g: 0.95,
                    b: 0.99,
                },
                callout_border: Rgb {
                    r: 0.28,
                    g: 0.44,
                    b: 0.78,
                },
                table_header_bg: Rgb {
                    r: 0.92,
                    g: 0.92,
                    b: 0.94,
                },
                table_border: Rgb {
                    r: 0.75,
                    g: 0.75,
                    b: 0.78,
                },
            },
            spacing: Spacing {
                block_gap: 0.4,
                list_indent_mm: 6.0,
                block_padding_mm: 2.5,
                table_cell_padding_mm: 1.5,
            },
            page_defaults: PageDefaults { margin_mm: 20.0 },
            decorations: Decorations {
                header: Some("{title}".to_string()),
                footer: Some("Page {page} of {total_pages}".to_string()),
                suppress_on_first_page: true,
                font_size_pt: 9.0,
                margin_mm: 10.0,
            },
            fonts: Fonts {
                regular: "LiberationSans-Regular.ttf".to_string(),
                bold: "LiberationSans-Bold.ttf".to_string(),
                italic: "LiberationSans-Italic.ttf".to_string(),
                mono: "LiberationSans-Regular.ttf".to_string(),
                fallbacks: vec!["DejaVuSans.ttf".to_string()],
            },
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
