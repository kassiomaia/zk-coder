//! Top-level [`PdfDocument`] value.

use serde::{Deserialize, Serialize};

use super::geometry::{Margins, Orientation, PageGeometry, PageSize};
use super::metadata::Metadata;
use super::section::Section;

/// Default A4 portrait geometry with 20 mm uniform margins.
const DEFAULT_GEOMETRY: PageGeometry = PageGeometry::new(
    PageSize::A4,
    Orientation::Portrait,
    Margins::const_uniform(20.0),
);

/// Top-level PDF document.
///
/// Owns metadata, geometry, and an ordered list of sections. Themes are
/// referenced by id so two documents can share one theme without copying
/// the style tree — the theme registry is introduced in T1.4.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct PdfDocument {
    /// Document metadata (title, author, ...).
    pub metadata: Metadata,
    /// Page size, orientation, and margins.
    pub geometry: PageGeometry,
    /// Optional theme identifier (resolved against a registry at render
    /// time).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    /// Ordered sections.
    #[serde(default)]
    pub sections: Vec<Section>,
}

impl PdfDocument {
    /// Builds a document with the given metadata and the default A4
    /// portrait geometry.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(metadata: Metadata) -> Self {
        Self {
            metadata,
            geometry: DEFAULT_GEOMETRY,
            theme: None,
            sections: Vec::new(),
        }
    }

    /// Overrides the page geometry.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_geometry(mut self, geometry: PageGeometry) -> Self {
        self.geometry = geometry;
        self
    }

    /// Sets a theme identifier on the document.
    #[must_use]
    pub fn with_theme(mut self, theme: impl Into<String>) -> Self {
        self.theme = Some(theme.into());
        self
    }

    /// Appends a section to the end of the document.
    pub fn push_section(&mut self, section: Section) {
        self.sections.push(section);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::documents::pdf::block::{Block, Paragraph};
    use crate::core::documents::pdf::section::{Heading, HeadingLevel};

    fn sample() -> PdfDocument {
        let mut doc = PdfDocument::new(Metadata::new("Report").unwrap())
            .with_theme("report")
            .with_geometry(PageGeometry::new(
                PageSize::A4,
                Orientation::Portrait,
                Margins::uniform(25.0).unwrap(),
            ));
        let heading = Heading::new(HeadingLevel::new(1).unwrap(), "Intro").unwrap();
        let mut section = Section::new(heading);
        section.push(Block::Paragraph(Paragraph::plain("first line").unwrap()));
        section.push(Block::PageBreak);
        section.push(Block::Paragraph(Paragraph::plain("second line").unwrap()));
        doc.push_section(section);
        doc
    }

    #[test]
    fn defaults_to_a4_portrait_with_20mm_margins() {
        let doc = PdfDocument::new(Metadata::new("T").unwrap());
        assert_eq!(doc.geometry.size, PageSize::A4);
        assert_eq!(doc.geometry.orientation, Orientation::Portrait);
        assert!((doc.geometry.margins.top_mm() - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn serde_roundtrip_full_document() {
        let doc = sample();
        let json = serde_json::to_string(&doc).unwrap();
        let back: PdfDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(doc, back);
    }

    #[test]
    fn theme_is_omitted_when_absent() {
        let doc = PdfDocument::new(Metadata::new("T").unwrap());
        let json = serde_json::to_string(&doc).unwrap();
        assert!(!json.contains("theme"));
    }
}
