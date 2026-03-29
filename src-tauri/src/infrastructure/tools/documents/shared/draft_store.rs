//! Draft store for incremental document building.
//!
//! Manages in-memory drafts for DOCX, PPTX, and PDF documents that are
//! being constructed incrementally by agents.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use uuid::Uuid;

/// Global draft store instance.
pub static DRAFT_STORE: OnceLock<Arc<Mutex<DraftStore>>> = OnceLock::new();

/// Get the global draft store, initializing if needed.
pub fn get_draft_store() -> Arc<Mutex<DraftStore>> {
    DRAFT_STORE
        .get_or_init(|| Arc::new(Mutex::new(DraftStore::new())))
        .clone()
}

/// In-memory storage for active document drafts.
pub struct DraftStore {
    /// DOCX drafts indexed by document_id
    docx_drafts: HashMap<String, DocxDraft>,
    /// PPTX drafts indexed by deck_id
    pptx_drafts: HashMap<String, PptxDraft>,
    /// PDF drafts indexed by pdf_id
    pdf_drafts: HashMap<String, PdfDraft>,
}

impl Default for DraftStore {
    fn default() -> Self {
        Self::new()
    }
}

impl DraftStore {
    /// Create a new empty draft store.
    pub fn new() -> Self {
        Self {
            docx_drafts: HashMap::new(),
            pptx_drafts: HashMap::new(),
            pdf_drafts: HashMap::new(),
        }
    }

    // === DOCX Draft Methods ===

    /// Create a new DOCX draft and return its ID.
    pub fn create_docx_draft(&mut self, filename: String) -> String {
        let id = format!("docx_{}", Uuid::new_v4());
        let draft = DocxDraft {
            document_id: id.clone(),
            filename,
            sections: Vec::new(),
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.docx_drafts.insert(id.clone(), draft);
        id
    }

    /// Get a mutable reference to a DOCX draft.
    pub fn get_docx_draft_mut(&mut self, doc_id: &str) -> Option<&mut DocxDraft> {
        self.docx_drafts.get_mut(doc_id)
    }

    /// Get an immutable reference to a DOCX draft.
    pub fn get_docx_draft(&self, doc_id: &str) -> Option<&DocxDraft> {
        self.docx_drafts.get(doc_id)
    }

    /// Add a section to a DOCX draft.
    pub fn add_docx_section(
        &mut self,
        doc_id: &str,
        content: DocxContent,
    ) -> Result<String, DraftError> {
        let draft = self
            .docx_drafts
            .get_mut(doc_id)
            .ok_or_else(|| DraftError::DraftNotFound(doc_id.to_string()))?;

        if draft.status != DraftStatus::Building {
            return Err(DraftError::NotBuilding(draft.status.clone()));
        }

        let section_id = format!("sec_{}", Uuid::new_v4());
        let index = draft.sections.len();
        draft.sections.push(DraftSection {
            id: section_id.clone(),
            index,
            content,
        });
        draft.updated_at = Utc::now();
        Ok(section_id)
    }

    /// Remove a DOCX draft and return it.
    pub fn remove_docx_draft(&mut self, doc_id: &str) -> Option<DocxDraft> {
        self.docx_drafts.remove(doc_id)
    }

    // === PPTX Draft Methods ===

    /// Create a new PPTX draft and return its ID.
    pub fn create_pptx_draft(&mut self, filename: String) -> String {
        let id = format!("pptx_{}", Uuid::new_v4());
        let draft = PptxDraft {
            deck_id: id.clone(),
            filename,
            slides: Vec::new(),
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.pptx_drafts.insert(id.clone(), draft);
        id
    }

    /// Get a mutable reference to a PPTX draft.
    pub fn get_pptx_draft_mut(&mut self, deck_id: &str) -> Option<&mut PptxDraft> {
        self.pptx_drafts.get_mut(deck_id)
    }

    /// Get an immutable reference to a PPTX draft.
    pub fn get_pptx_draft(&self, deck_id: &str) -> Option<&PptxDraft> {
        self.pptx_drafts.get(deck_id)
    }

    /// Add a slide to a PPTX draft.
    pub fn add_pptx_slide(
        &mut self,
        deck_id: &str,
        slide: PptxSlide,
    ) -> Result<String, DraftError> {
        let draft = self
            .pptx_drafts
            .get_mut(deck_id)
            .ok_or_else(|| DraftError::DraftNotFound(deck_id.to_string()))?;

        if draft.status != DraftStatus::Building {
            return Err(DraftError::NotBuilding(draft.status.clone()));
        }

        let slide_id = format!("slide_{}", Uuid::new_v4());
        let index = draft.slides.len();
        draft.slides.push(DraftSlide {
            id: slide_id.clone(),
            index,
            slide,
        });
        draft.updated_at = Utc::now();
        Ok(slide_id)
    }

    /// Remove a PPTX draft and return it.
    pub fn remove_pptx_draft(&mut self, deck_id: &str) -> Option<PptxDraft> {
        self.pptx_drafts.remove(deck_id)
    }

    // === PDF Draft Methods ===

    /// Create a new PDF draft and return its ID.
    pub fn create_pdf_draft(&mut self, filename: String, page_size: Option<PdfPageSize>) -> String {
        let id = format!("pdf_{}", Uuid::new_v4());
        let draft = PdfDraft {
            pdf_id: id.clone(),
            filename,
            pages: Vec::new(),
            page_size: page_size.unwrap_or_default(),
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.pdf_drafts.insert(id.clone(), draft);
        id
    }

    /// Get a mutable reference to a PDF draft.
    pub fn get_pdf_draft_mut(&mut self, pdf_id: &str) -> Option<&mut PdfDraft> {
        self.pdf_drafts.get_mut(pdf_id)
    }

    /// Get an immutable reference to a PDF draft.
    pub fn get_pdf_draft(&self, pdf_id: &str) -> Option<&PdfDraft> {
        self.pdf_drafts.get(pdf_id)
    }

    /// Add a page to a PDF draft.
    pub fn add_pdf_page(
        &mut self,
        pdf_id: &str,
        content: PdfPageContent,
    ) -> Result<String, DraftError> {
        let draft = self
            .pdf_drafts
            .get_mut(pdf_id)
            .ok_or_else(|| DraftError::DraftNotFound(pdf_id.to_string()))?;

        if draft.status != DraftStatus::Building {
            return Err(DraftError::NotBuilding(draft.status.clone()));
        }

        let page_id = format!("page_{}", Uuid::new_v4());
        let index = draft.pages.len();
        draft.pages.push(DraftPage {
            id: page_id.clone(),
            index,
            content,
        });
        draft.updated_at = Utc::now();
        Ok(page_id)
    }

    /// Remove a PDF draft and return it.
    pub fn remove_pdf_draft(&mut self, pdf_id: &str) -> Option<PdfDraft> {
        self.pdf_drafts.remove(pdf_id)
    }

    // === Cleanup Methods ===

    /// Remove drafts that have been abandoned (in Building status for too long).
    pub fn cleanup_expired_drafts(&mut self, timeout: Duration) -> usize {
        let now = Utc::now();
        let mut removed = 0;

        // Clean DOCX drafts
        self.docx_drafts.retain(|_, draft| {
            if draft.status == DraftStatus::Building {
                let age = now.signed_duration_since(draft.updated_at);
                if age.num_seconds() > timeout.as_secs() as i64 {
                    removed += 1;
                    return false;
                }
            }
            true
        });

        // Clean PPTX drafts
        self.pptx_drafts.retain(|_, draft| {
            if draft.status == DraftStatus::Building {
                let age = now.signed_duration_since(draft.updated_at);
                if age.num_seconds() > timeout.as_secs() as i64 {
                    removed += 1;
                    return false;
                }
            }
            true
        });

        // Clean PDF drafts
        self.pdf_drafts.retain(|_, draft| {
            if draft.status == DraftStatus::Building {
                let age = now.signed_duration_since(draft.updated_at);
                if age.num_seconds() > timeout.as_secs() as i64 {
                    removed += 1;
                    return false;
                }
            }
            true
        });

        removed
    }

    /// Get counts of all drafts by status.
    pub fn get_counts(&self) -> DraftCounts {
        let mut counts = DraftCounts::default();
        counts.docx_building = self
            .docx_drafts
            .values()
            .filter(|d| d.status == DraftStatus::Building)
            .count();
        counts.docx_finalizing = self
            .docx_drafts
            .values()
            .filter(|d| d.status == DraftStatus::Finalizing)
            .count();
        counts.pptx_building = self
            .pptx_drafts
            .values()
            .filter(|d| d.status == DraftStatus::Building)
            .count();
        counts.pptx_finalizing = self
            .pptx_drafts
            .values()
            .filter(|d| d.status == DraftStatus::Finalizing)
            .count();
        counts.pdf_building = self
            .pdf_drafts
            .values()
            .filter(|d| d.status == DraftStatus::Building)
            .count();
        counts.pdf_finalizing = self
            .pdf_drafts
            .values()
            .filter(|d| d.status == DraftStatus::Finalizing)
            .count();
        counts
    }
}

// === Draft Types ===

/// Status of a document draft.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DraftStatus {
    /// Draft is actively being built
    Building,
    /// Draft is being finalized (generating actual file)
    Finalizing,
    /// Draft is complete and stored as artifact
    Complete,
    /// Draft was abandoned by user
    Abandoned,
}

/// A DOCX draft being incrementally built.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocxDraft {
    /// Unique identifier for the document.
    pub document_id: String,
    /// The filename for the document.
    pub filename: String,
    /// Sections contained in the document.
    pub sections: Vec<DraftSection>,
    /// Current status of the draft.
    pub status: DraftStatus,
    /// Timestamp when the draft was created.
    pub created_at: DateTime<Utc>,
    /// Timestamp when the draft was last updated.
    pub updated_at: DateTime<Utc>,
}

/// A PPTX draft being incrementally built.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PptxDraft {
    /// Unique identifier for the presentation deck.
    pub deck_id: String,
    /// The filename for the presentation.
    pub filename: String,
    /// Slides contained in the presentation.
    pub slides: Vec<DraftSlide>,
    /// Current status of the draft.
    pub status: DraftStatus,
    /// Timestamp when the draft was created.
    pub created_at: DateTime<Utc>,
    /// Timestamp when the draft was last updated.
    pub updated_at: DateTime<Utc>,
}

/// A PDF draft being incrementally built.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfDraft {
    /// Unique identifier for the PDF.
    pub pdf_id: String,
    /// The filename for the PDF.
    pub filename: String,
    /// Pages contained in the PDF.
    pub pages: Vec<DraftPage>,
    /// The page size for the PDF.
    pub page_size: PdfPageSize,
    /// Current status of the draft.
    pub status: DraftStatus,
    /// Timestamp when the draft was created.
    pub created_at: DateTime<Utc>,
    /// Timestamp when the draft was last updated.
    pub updated_at: DateTime<Utc>,
}

/// A section within a DOCX draft.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftSection {
    /// Unique identifier for the section.
    pub id: String,
    /// Index of the section in the document.
    pub index: usize,
    /// Content of the section.
    pub content: DocxContent,
}

/// A slide within a PPTX draft.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftSlide {
    /// Unique identifier for the slide.
    pub id: String,
    /// Index of the slide in the presentation.
    pub index: usize,
    /// The slide content.
    pub slide: PptxSlide,
}

/// A page within a PDF draft.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftPage {
    /// Unique identifier for the page.
    pub id: String,
    /// Index of the page in the PDF.
    pub index: usize,
    /// Content of the page.
    pub content: PdfPageContent,
}

// === Content Types ===

/// Content types for DOCX sections.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DocxContent {
    /// A heading with a specified level.
    Heading {
        /// The heading level (1-9).
        level: u8,
        /// The heading text.
        text: String,
        /// Whether to insert a page break before this heading.
        page_break_before: bool,
    },
    /// A paragraph of text.
    Paragraph {
        /// The paragraph text.
        text: String,
        /// Optional paragraph style.
        style: Option<String>,
    },
    /// A bullet list.
    BulletList {
        /// List items.
        items: Vec<String>,
    },
    /// A numbered list.
    NumberedList {
        /// List items.
        items: Vec<String>,
        /// Number to start counting from.
        start_at: Option<u32>,
    },
    /// A table with headers and rows.
    Table {
        /// Column headers.
        headers: Vec<String>,
        /// Table rows.
        rows: Vec<Vec<String>>,
        /// Optional table caption.
        caption: Option<String>,
    },
    /// An image.
    Image {
        /// Path to the image file.
        path: String,
        /// Optional image caption.
        caption: Option<String>,
        /// Optional image width.
        width: Option<String>,
    },
    /// A page break.
    PageBreak,
    /// A horizontal rule.
    HorizontalRule,
}

/// A slide in a PPTX presentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PptxSlide {
    /// The layout type for the slide.
    pub layout: SlideLayout,
    /// The title of the slide.
    pub title: String,
    /// Content elements on the slide.
    pub content: Vec<SlideContent>,
    /// Speaker notes for the slide.
    pub notes: Option<String>,
}

/// Slide layout types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SlideLayout {
    /// Title slide layout.
    TitleSlide,
    /// Title and content layout.
    TitleAndContent,
    /// Section header layout.
    SectionHeader,
    /// Two column layout.
    TwoColumn,
    /// Two content areas layout.
    TwoContent,
    /// Comparison layout.
    Comparison,
    /// Content with caption layout.
    ContentWithCaption,
    /// Blank layout.
    Blank,
}

/// Content within a slide.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SlideContent {
    /// Title content element.
    Title,
    /// Subtitle content element.
    Subtitle,
    /// A text box with content.
    TextBox {
        /// The text content.
        text: String,
        /// Optional position on the slide.
        position: Option<String>,
    },
    /// A bullet list.
    BulletList {
        /// List items.
        items: Vec<String>,
        /// Nesting level of the list.
        level: u8,
    },
    /// A numbered list with ordered items.
    NumberedList {
        /// List items.
        items: Vec<String>,
    },
    /// A table with headers and rows.
    Table {
        /// Column headers.
        headers: Vec<String>,
        /// Table rows.
        rows: Vec<Vec<String>>,
    },
    /// An image with optional caption.
    Image {
        /// Path to the image file.
        path: String,
        /// Optional image caption.
        caption: Option<String>,
    },
    /// A placeholder for a chart with specified data type.
    ChartPlaceholder {
        /// Title for the chart.
        title: String,
        /// Type of chart data.
        data_type: String,
    },
}

/// Content types for PDF pages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PdfPageContent {
    /// Markdown formatted content.
    Markdown {
        /// The markdown content.
        content: String,
    },
    /// HTML formatted content with optional styles.
    Html {
        /// The HTML content.
        content: String,
        /// Optional CSS styles.
        styles: Option<String>,
    },
    /// Plain text content.
    Text {
        /// The text content.
        content: String,
    },
    /// An image with optional caption.
    Image {
        /// Path to the image file.
        path: String,
        /// Optional image caption.
        caption: Option<String>,
    },
}

/// PDF page sizes.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub enum PdfPageSize {
    /// A4 size (210 x 297 mm).
    #[default]
    A4,
    /// Letter size (8.5 x 11 inches).
    Letter,
    /// Legal size (8.5 x 14 inches).
    Legal,
    /// A3 size (297 x 420 mm).
    A3,
}

// === Error Types ===

/// Errors that can occur in draft operations.
#[derive(Debug, thiserror::Error)]
pub enum DraftError {
    /// The requested draft was not found.
    #[error("Draft not found: {0}")]
    DraftNotFound(String),

    /// The draft is not in Building status and cannot be modified.
    #[error("Draft is not in Building status, current status: {0:?}")]
    NotBuilding(DraftStatus),

    /// The requested section was not found.
    #[error("Section not found: {0}")]
    SectionNotFound(String),

    /// The requested slide was not found.
    #[error("Slide not found: {0}")]
    SlideNotFound(String),

    /// The requested page was not found.
    #[error("Page not found: {0}")]
    PageNotFound(String),
}

/// Counts of drafts by type and status.
#[derive(Debug, Default, Clone, Copy)]
pub struct DraftCounts {
    /// Number of DOCX drafts currently being built.
    pub docx_building: usize,
    /// Number of DOCX drafts currently being finalized.
    pub docx_finalizing: usize,
    /// Number of PPTX drafts currently being built.
    pub pptx_building: usize,
    /// Number of PPTX drafts currently being finalized.
    pub pptx_finalizing: usize,
    /// Number of PDF drafts currently being built.
    pub pdf_building: usize,
    /// Number of PDF drafts currently being finalized.
    pub pdf_finalizing: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_docx_draft() {
        let mut store = DraftStore::new();
        let id = store.create_docx_draft("test.docx".to_string());

        let draft = store.get_docx_draft(&id).unwrap();
        assert_eq!(draft.filename, "test.docx");
        assert_eq!(draft.sections.len(), 0);
        assert_eq!(draft.status, DraftStatus::Building);
    }

    #[test]
    fn test_add_docx_section() {
        let mut store = DraftStore::new();
        let id = store.create_docx_draft("test.docx".to_string());

        let content = DocxContent::Heading {
            level: 1,
            text: "Title".to_string(),
            page_break_before: false,
        };

        let section_id = store.add_docx_section(&id, content).unwrap();

        let draft = store.get_docx_draft(&id).unwrap();
        assert_eq!(draft.sections.len(), 1);
        assert_eq!(draft.sections[0].id, section_id);
        assert_eq!(draft.sections[0].index, 0);
    }

    #[test]
    fn test_add_multiple_docx_sections() {
        let mut store = DraftStore::new();
        let id = store.create_docx_draft("test.docx".to_string());

        store
            .add_docx_section(
                &id,
                DocxContent::Heading {
                    level: 1,
                    text: "Title".to_string(),
                    page_break_before: false,
                },
            )
            .unwrap();

        store
            .add_docx_section(
                &id,
                DocxContent::Paragraph {
                    text: "Content".to_string(),
                    style: None,
                },
            )
            .unwrap();

        let draft = store.get_docx_draft(&id).unwrap();
        assert_eq!(draft.sections.len(), 2);
        assert_eq!(draft.sections[0].index, 0);
        assert_eq!(draft.sections[1].index, 1);
    }

    #[test]
    fn test_create_pptx_draft() {
        let mut store = DraftStore::new();
        let id = store.create_pptx_draft("presentation.pptx".to_string());

        let draft = store.get_pptx_draft(&id).unwrap();
        assert_eq!(draft.filename, "presentation.pptx");
        assert_eq!(draft.slides.len(), 0);
    }

    #[test]
    fn test_add_pptx_slide() {
        let mut store = DraftStore::new();
        let id = store.create_pptx_draft("presentation.pptx".to_string());

        let slide = PptxSlide {
            layout: SlideLayout::TitleSlide,
            title: "Title".to_string(),
            content: vec![],
            notes: None,
        };

        let slide_id = store.add_pptx_slide(&id, slide).unwrap();

        let draft = store.get_pptx_draft(&id).unwrap();
        assert_eq!(draft.slides.len(), 1);
        assert_eq!(draft.slides[0].id, slide_id);
    }

    #[test]
    fn test_create_pdf_draft() {
        let mut store = DraftStore::new();
        let id = store.create_pdf_draft("document.pdf".to_string(), None);

        let draft = store.get_pdf_draft(&id).unwrap();
        assert_eq!(draft.filename, "document.pdf");
        assert_eq!(draft.pages.len(), 0);
        assert_eq!(draft.page_size, PdfPageSize::A4);
    }

    #[test]
    fn test_add_pdf_page() {
        let mut store = DraftStore::new();
        let id = store.create_pdf_draft("document.pdf".to_string(), None);

        let content = PdfPageContent::Text {
            content: "Page content".to_string(),
        };

        let page_id = store.add_pdf_page(&id, content).unwrap();

        let draft = store.get_pdf_draft(&id).unwrap();
        assert_eq!(draft.pages.len(), 1);
        assert_eq!(draft.pages[0].id, page_id);
    }

    #[test]
    fn test_draft_not_found_error() {
        let store = DraftStore::new();
        let result = store.get_docx_draft("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_remove_docx_draft() {
        let mut store = DraftStore::new();
        let id = store.create_docx_draft("test.docx".to_string());

        let removed = store.remove_docx_draft(&id).unwrap();
        assert_eq!(removed.filename, "test.docx");

        assert!(store.get_docx_draft(&id).is_none());
    }

    #[test]
    fn test_get_counts() {
        let mut store = DraftStore::new();

        store.create_docx_draft("test1.docx".to_string());
        store.create_docx_draft("test2.docx".to_string());
        store.create_pptx_draft("test.pptx".to_string());
        store.create_pdf_draft("test.pdf".to_string(), None);

        let counts = store.get_counts();
        assert_eq!(counts.docx_building, 2);
        assert_eq!(counts.pptx_building, 1);
        assert_eq!(counts.pdf_building, 1);
    }
}
