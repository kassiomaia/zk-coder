//! Page size, orientation, and margins.

use serde::{Deserialize, Serialize};

use super::error::PdfModelError;

/// Physical page size.
///
/// Standard variants map to ISO and US paper sizes. `Custom` carries
/// explicit millimetre dimensions so the model remains engine-agnostic;
/// the renderer converts to its native units.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
pub enum PageSize {
    /// ISO A4 (210 × 297 mm).
    A4,
    /// ISO A3 (297 × 420 mm).
    A3,
    /// US Letter (215.9 × 279.4 mm).
    Letter,
    /// US Legal (215.9 × 355.6 mm).
    Legal,
    /// Custom size with explicit millimetre dimensions.
    Custom {
        /// Width in millimetres.
        width_mm: f32,
        /// Height in millimetres.
        height_mm: f32,
    },
}

impl PageSize {
    /// Returns `(width_mm, height_mm)` for this page size.
    #[must_use]
    pub const fn dimensions_mm(&self) -> (f32, f32) {
        match *self {
            Self::A4 => (210.0, 297.0),
            Self::A3 => (297.0, 420.0),
            Self::Letter => (215.9, 279.4),
            Self::Legal => (215.9, 355.6),
            Self::Custom {
                width_mm,
                height_mm,
            } => (width_mm, height_mm),
        }
    }

    /// Builds a `Custom` page size.
    ///
    /// # Errors
    /// Returns [`PdfModelError::InvalidPageDimension`] if either value is
    /// non-finite or not strictly positive.
    pub fn custom(width_mm: f32, height_mm: f32) -> Result<Self, PdfModelError> {
        if !width_mm.is_finite() || width_mm <= 0.0 {
            return Err(PdfModelError::InvalidPageDimension(width_mm));
        }
        if !height_mm.is_finite() || height_mm <= 0.0 {
            return Err(PdfModelError::InvalidPageDimension(height_mm));
        }
        Ok(Self::Custom {
            width_mm,
            height_mm,
        })
    }
}

/// Page orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    /// Portrait (taller than wide).
    Portrait,
    /// Landscape (wider than tall).
    Landscape,
}

/// Per-side page margins, in millimetres.
///
/// Values are validated to be finite and non-negative at construction.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq, clippy::struct_field_names)]
pub struct Margins {
    top_mm: f32,
    right_mm: f32,
    bottom_mm: f32,
    left_mm: f32,
}

impl Margins {
    /// Builds margins from four explicit side values.
    ///
    /// # Errors
    /// Returns [`PdfModelError::InvalidMargin`] if any value is non-finite
    /// or negative.
    pub fn new(
        top_mm: f32,
        right_mm: f32,
        bottom_mm: f32,
        left_mm: f32,
    ) -> Result<Self, PdfModelError> {
        for v in [top_mm, right_mm, bottom_mm, left_mm] {
            if !v.is_finite() || v < 0.0 {
                return Err(PdfModelError::InvalidMargin(v));
            }
        }
        Ok(Self {
            top_mm,
            right_mm,
            bottom_mm,
            left_mm,
        })
    }

    /// Builds margins with the same value on every side.
    ///
    /// # Errors
    /// Returns [`PdfModelError::InvalidMargin`] if `mm` is non-finite or
    /// negative.
    pub fn uniform(mm: f32) -> Result<Self, PdfModelError> {
        Self::new(mm, mm, mm, mm)
    }

    /// `const fn` constructor used to build compile-time defaults.
    ///
    /// Skips the finite / non-negative check because `const fn` cannot
    /// call `f32::is_finite`. Only use from internal `const` contexts with
    /// literal values that are known to be valid.
    pub(crate) const fn const_uniform(mm: f32) -> Self {
        Self {
            top_mm: mm,
            right_mm: mm,
            bottom_mm: mm,
            left_mm: mm,
        }
    }

    /// Returns the top margin in millimetres.
    #[must_use]
    pub const fn top_mm(self) -> f32 {
        self.top_mm
    }

    /// Returns the right margin in millimetres.
    #[must_use]
    pub const fn right_mm(self) -> f32 {
        self.right_mm
    }

    /// Returns the bottom margin in millimetres.
    #[must_use]
    pub const fn bottom_mm(self) -> f32 {
        self.bottom_mm
    }

    /// Returns the left margin in millimetres.
    #[must_use]
    pub const fn left_mm(self) -> f32 {
        self.left_mm
    }
}

/// Page geometry: size + orientation + margins.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct PageGeometry {
    /// Page size (A4, Letter, ...).
    pub size: PageSize,
    /// Page orientation.
    pub orientation: Orientation,
    /// Per-side margins.
    pub margins: Margins,
}

impl PageGeometry {
    /// Builds a `PageGeometry` from its three components.
    #[must_use]
    pub const fn new(size: PageSize, orientation: Orientation, margins: Margins) -> Self {
        Self {
            size,
            orientation,
            margins,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_dimensions() {
        assert_eq!(PageSize::A4.dimensions_mm(), (210.0, 297.0));
    }

    #[test]
    fn custom_rejects_negative_width() {
        assert!(matches!(
            PageSize::custom(-1.0, 100.0),
            Err(PdfModelError::InvalidPageDimension(_))
        ));
    }

    #[test]
    fn custom_rejects_nan() {
        assert!(matches!(
            PageSize::custom(f32::NAN, 100.0),
            Err(PdfModelError::InvalidPageDimension(_))
        ));
    }

    #[test]
    fn margins_reject_negative_value() {
        assert!(matches!(
            Margins::new(10.0, 10.0, -1.0, 10.0),
            Err(PdfModelError::InvalidMargin(_))
        ));
    }

    #[test]
    fn margins_accept_zero() {
        let m = Margins::uniform(0.0).unwrap();
        assert!((m.top_mm() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn serde_roundtrip_geometry() {
        let g = PageGeometry::new(
            PageSize::A4,
            Orientation::Portrait,
            Margins::uniform(20.0).unwrap(),
        );
        let json = serde_json::to_string(&g).unwrap();
        let back: PageGeometry = serde_json::from_str(&json).unwrap();
        assert_eq!(g, back);
    }

    #[test]
    fn serde_roundtrip_custom_page_size() {
        let g = PageGeometry::new(
            PageSize::custom(200.0, 250.0).unwrap(),
            Orientation::Landscape,
            Margins::new(10.0, 15.0, 10.0, 15.0).unwrap(),
        );
        let json = serde_json::to_string(&g).unwrap();
        let back: PageGeometry = serde_json::from_str(&json).unwrap();
        assert_eq!(g, back);
    }
}
