//! Shared types and utilities for document tools.

pub mod artifact_store;
pub mod draft_store;
pub mod validation;

pub use artifact_store::{
    Artifact, ArtifactError, ArtifactMetadata, ArtifactRef, ArtifactStore, ArtifactType,
};
pub use draft_store::{
    get_draft_store, DocxContent, DocxDraft, DraftCounts, DraftError, DraftPage, DraftSection,
    DraftSlide, DraftStatus, DraftStore, PdfDraft, PdfInsertPosition, PdfPageContent, PdfPageSize,
    PptxDraft, PptxSlide, SlideContent, SlideLayout,
};
pub use validation::{
    validate_file_exists, validate_file_size, validate_magic_bytes, DocumentFileType,
    ValidationCheck, ValidationLevel, ValidationReport,
};

use std::sync::{Arc, Mutex};

/// Global draft store instance type.
pub type GlobalDraftStore = Arc<Mutex<DraftStore>>;

/// Global artifact store instance type.
pub type GlobalArtifactStore = Arc<Mutex<ArtifactStore>>;

/// Extension trait to make draft store accessible from tools.
pub trait DraftStoreExt {
    /// Get the global draft store instance.
    fn get_draft_store() -> GlobalDraftStore;
}

impl DraftStoreExt for DraftStore {
    fn get_draft_store() -> GlobalDraftStore {
        crate::infrastructure::tools::documents::shared::draft_store::get_draft_store()
    }
}
