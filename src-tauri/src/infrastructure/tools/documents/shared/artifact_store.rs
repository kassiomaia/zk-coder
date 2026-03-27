//! Artifact store for finalized document artifacts.
//!
//! Manages persistent storage for DOCX, PPTX, and PDF documents
//! that have been finalized by agents.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during artifact operations.
#[derive(Debug, Error)]
pub enum ArtifactError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Artifact not found: {0}")]
    NotFound(String),

    #[error("Path validation failed: {0}")]
    PathValidation(String),

    #[error("Invalid artifact data")]
    InvalidData,

    #[error("Size limit exceeded: {size} bytes (max: {max} bytes)")]
    SizeLimitExceeded { size: u64, max: u64 },
}

/// Type of document artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArtifactType {
    #[serde(rename = "Docx")]
    Docx,
    #[serde(rename = "Pptx")]
    Pptx,
    #[serde(rename = "Pdf")]
    Pdf,
}

impl ArtifactType {
    /// Get the file extension for this artifact type.
    pub fn extension(&self) -> &str {
        match self {
            ArtifactType::Docx => ".docx",
            ArtifactType::Pptx => ".pptx",
            ArtifactType::Pdf => ".pdf",
        }
    }

    /// Get the MIME type for this artifact type.
    pub fn mime_type(&self) -> &str {
        match self {
            ArtifactType::Docx => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            }
            ArtifactType::Pptx => {
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            }
            ArtifactType::Pdf => "application/pdf",
        }
    }

    /// Get the maximum allowed size in bytes.
    pub fn max_size(&self) -> u64 {
        match self {
            ArtifactType::Docx => 50 * 1024 * 1024,  // 50MB
            ArtifactType::Pptx => 100 * 1024 * 1024, // 100MB
            ArtifactType::Pdf => 100 * 1024 * 1024,  // 100MB
        }
    }
}

/// Metadata about an artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    /// Optional title
    pub title: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Agent that generated this artifact
    pub generated_by: Option<String>,
    /// Original user prompt that led to creation
    pub source_prompt: Option<String>,
    /// Template used (if any)
    pub template: Option<String>,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Default for ArtifactMetadata {
    fn default() -> Self {
        Self {
            title: None,
            description: None,
            generated_by: None,
            source_prompt: None,
            template: None,
            tags: Vec::new(),
        }
    }
}

/// Reference to an artifact (lightweight, for listing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRef {
    /// Unique artifact ID
    pub id: String,
    /// Session ID this artifact belongs to
    pub session_id: String,
    /// Type of artifact
    pub artifact_type: ArtifactType,
    /// Filename
    pub filename: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Optional metadata
    #[serde(default)]
    pub metadata: ArtifactMetadata,
    /// Path to the artifact file
    pub path: PathBuf,
}

/// Full artifact data with content.
#[derive(Debug, Clone)]
pub struct Artifact {
    /// Artifact reference data
    pub reference: ArtifactRef,
    /// File content bytes
    pub content: Vec<u8>,
}

/// Storage for finalized document artifacts.
pub struct ArtifactStore {
    /// Base directory for artifact storage
    base_path: PathBuf,
    /// In-memory index of artifacts by ID
    artifacts: HashMap<String, ArtifactRef>,
    /// Index of artifacts by session ID
    session_artifacts: HashMap<String, Vec<String>>,
}

impl Default for ArtifactStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtifactStore {
    /// Create a new artifact store with default path.
    pub fn new() -> Self {
        let base_path = Self::default_base_path();
        Self::with_path(base_path)
    }

    /// Create a new artifact store with a custom base path.
    pub fn with_path(base_path: PathBuf) -> Self {
        // Ensure base directory exists
        fs::create_dir_all(&base_path).ok();

        Self {
            base_path,
            artifacts: HashMap::new(),
            session_artifacts: HashMap::new(),
        }
    }

    /// Get the default base path for artifacts.
    fn default_base_path() -> PathBuf {
        // Use ~/.coder/artifacts
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join(".coder").join("artifacts"))
            .or_else(|| {
                std::env::var("USERPROFILE")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".coder").join("artifacts"))
            })
            .unwrap_or_else(|| PathBuf::from("/tmp/coder/artifacts"))
    }

    /// Get the path for a session's artifacts.
    fn session_path(&self, session_id: &str) -> PathBuf {
        self.base_path.join(session_id)
    }

    /// Get the path for a specific artifact.
    fn artifact_path(&self, session_id: &str, artifact_id: &str) -> PathBuf {
        self.session_path(session_id).join(artifact_id)
    }

    /// Ensure the session directory exists.
    fn ensure_session_dir(&self, session_id: &str) -> Result<PathBuf, ArtifactError> {
        let path = self.session_path(session_id);
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    /// Validate that a path is within the allowed base directory.
    fn validate_path(&self, path: &Path) -> Result<(), ArtifactError> {
        let canonical = path
            .canonicalize()
            .unwrap_or_else(|_| path.to_path_buf());
        let base = self
            .base_path
            .canonicalize()
            .unwrap_or_else(|_| self.base_path.clone());

        if !canonical.starts_with(&base) {
            return Err(ArtifactError::PathValidation(
                "Path escapes base directory".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate file magic bytes for the given type.
    fn validate_magic_bytes(content: &[u8], artifact_type: &ArtifactType) -> Result<(), ArtifactError> {
        if content.len() < 4 {
            return Err(ArtifactError::InvalidData);
        }

        let valid = match artifact_type {
            ArtifactType::Docx | ArtifactType::Pptx => {
                // Both are ZIP files (PK\x03\x04)
                &content[0..4] == b"PK\x03\x04" || &content[0..4] == b"PK\x05\x06"
            }
            ArtifactType::Pdf => &content[0..4] == b"%PDF",
        };

        if !valid {
            return Err(ArtifactError::InvalidData);
        }

        Ok(())
    }

    /// Save a new artifact.
    pub fn save_artifact(
        &mut self,
        session_id: &str,
        artifact_type: ArtifactType,
        filename: String,
        content: Vec<u8>,
        metadata: ArtifactMetadata,
    ) -> Result<ArtifactRef, ArtifactError> {
        // Validate size
        let size = content.len() as u64;
        let max_size = artifact_type.max_size();
        if size > max_size {
            return Err(ArtifactError::SizeLimitExceeded {
                size,
                max: max_size,
            });
        }

        // Validate magic bytes
        Self::validate_magic_bytes(&content, &artifact_type)?;

        // Ensure filename has correct extension
        let filename = if filename.ends_with(artifact_type.extension()) {
            filename
        } else {
            format!("{}{}", filename, artifact_type.extension())
        };

        // Create artifact ID and directory
        let id = format!("art_{}", Uuid::new_v4());
        let artifact_dir = self.artifact_path(session_id, &id);
        self.ensure_session_dir(session_id)?;

        // Create artifact-specific subdirectory
        fs::create_dir_all(&artifact_dir)?;

        // Write artifact file
        let file_path = artifact_dir.join(&filename);
        fs::write(&file_path, content)?;

        // Validate the written path
        self.validate_path(&file_path)?;

        // Create artifact reference
        let reference = ArtifactRef {
            id: id.clone(),
            session_id: session_id.to_string(),
            artifact_type,
            filename,
            size_bytes: size,
            created_at: Utc::now(),
            metadata,
            path: file_path,
        };

        // Update indexes
        self.artifacts.insert(id.clone(), reference.clone());
        self.session_artifacts
            .entry(session_id.to_string())
            .or_insert_with(Vec::new)
            .push(id.clone());

        Ok(reference)
    }

    /// Get an artifact reference by ID.
    pub fn get_artifact_ref(&self, id: &str) -> Option<&ArtifactRef> {
        self.artifacts.get(id)
    }

    /// Get full artifact data including content.
    pub fn get_artifact(&self, id: &str) -> Result<Artifact, ArtifactError> {
        let reference = self
            .artifacts
            .get(id)
            .ok_or_else(|| ArtifactError::NotFound(id.to_string()))?;

        // Validate path before reading
        self.validate_path(&reference.path)?;

        let content = fs::read(&reference.path)?;

        // Verify magic bytes match expected type
        Self::validate_magic_bytes(&content, &reference.artifact_type)?;

        Ok(Artifact {
            reference: reference.clone(),
            content,
        })
    }

    /// Get artifact content as bytes.
    pub fn get_artifact_content(&self, id: &str) -> Result<Vec<u8>, ArtifactError> {
        let reference = self
            .artifacts
            .get(id)
            .ok_or_else(|| ArtifactError::NotFound(id.to_string()))?;

        self.validate_path(&reference.path)?;
        fs::read(&reference.path).map_err(Into::into)
    }

    /// List all artifacts for a session.
    pub fn list_session_artifacts(&self, session_id: &str) -> Vec<ArtifactRef> {
        if let Some(ids) = self.session_artifacts.get(session_id) {
            ids.iter()
                .filter_map(|id| self.artifacts.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// List all artifacts across all sessions.
    pub fn list_all_artifacts(&self) -> Vec<ArtifactRef> {
        self.artifacts.values().cloned().collect()
    }

    /// Delete an artifact.
    pub fn delete_artifact(&mut self, session_id: &str, id: &str) -> Result<(), ArtifactError> {
        let reference = self
            .artifacts
            .get(id)
            .ok_or_else(|| ArtifactError::NotFound(id.to_string()))?;

        // Verify session matches
        if reference.session_id != session_id {
            return Err(ArtifactError::NotFound(id.to_string()));
        }

        // Validate path before deletion
        self.validate_path(&reference.path)?;

        // Remove file and directory
        let artifact_dir = reference.path.parent();
        if let Some(dir) = artifact_dir {
            fs::remove_file(&reference.path)?;
            // Try to remove the directory if empty
            fs::remove_dir(dir).ok();
        }

        // Remove from indexes
        self.artifacts.remove(id);
        if let Some(artifacts) = self.session_artifacts.get_mut(session_id) {
            artifacts.retain(|a| a != id);
            if artifacts.is_empty() {
                self.session_artifacts.remove(session_id);
            }
        }

        Ok(())
    }

    /// Update artifact metadata.
    pub fn update_metadata(
        &mut self,
        id: &str,
        metadata: ArtifactMetadata,
    ) -> Result<ArtifactRef, ArtifactError> {
        let reference = self
            .artifacts
            .get_mut(id)
            .ok_or_else(|| ArtifactError::NotFound(id.to_string()))?;

        reference.metadata = metadata;
        Ok(reference.clone())
    }

    /// Clean up old artifacts from a session.
    pub fn cleanup_session(&mut self, session_id: &str) -> Result<usize, ArtifactError> {
        let session_path = self.session_path(session_id);

        // Validate path is within base
        self.validate_path(&session_path)?;

        // Remove session directory
        let count = if session_path.exists() {
            fs::remove_dir_all(&session_path)?;
            self.session_artifacts.remove(session_id).map(|v| v.len()).unwrap_or(0)
        } else {
            0
        };

        // Remove artifacts from index
        self.artifacts
            .retain(|_, r| r.session_id != session_id);

        Ok(count)
    }

    /// Get the base path.
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Get total artifact count.
    pub fn count(&self) -> usize {
        self.artifacts.len()
    }

    /// Get artifact count for a session.
    pub fn session_count(&self, session_id: &str) -> usize {
        self.session_artifacts
            .get(session_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_store() -> (ArtifactStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store = ArtifactStore::with_path(temp_dir.path().to_path_buf());
        (store, temp_dir)
    }

    #[test]
    fn test_save_and_get_artifact() {
        let mut store = create_test_store().0;

        // Create a simple DOCX-like content (ZIP magic bytes)
        let content = b"PK\x03\x04test content".to_vec();
        let metadata = ArtifactMetadata {
            title: Some("Test Document".to_string()),
            ..Default::default()
        };

        let reference = store
            .save_artifact(
                "session_1",
                ArtifactType::Docx,
                "test.docx".to_string(),
                content,
                metadata,
            )
            .unwrap();

        assert_eq!(reference.filename, "test.docx");
        assert_eq!(reference.artifact_type, ArtifactType::Docx);
        assert!(reference.path.exists());
    }

    #[test]
    fn test_list_session_artifacts() {
        let mut store = create_test_store().0;

        store
            .save_artifact(
                "session_1",
                ArtifactType::Pdf,
                "doc1.pdf".to_string(),
                b"%PDFtest".to_vec(),
                ArtifactMetadata::default(),
            )
            .unwrap();

        store
            .save_artifact(
                "session_1",
                ArtifactType::Pdf,
                "doc2.pdf".to_string(),
                b"%PDFtest2".to_vec(),
                ArtifactMetadata::default(),
            )
            .unwrap();

        store
            .save_artifact(
                "session_2",
                ArtifactType::Docx,
                "doc.docx".to_string(),
                b"PK\x03\x04test".to_vec(),
                ArtifactMetadata::default(),
            )
            .unwrap();

        let session1_artifacts = store.list_session_artifacts("session_1");
        assert_eq!(session1_artifacts.len(), 2);

        let session2_artifacts = store.list_session_artifacts("session_2");
        assert_eq!(session2_artifacts.len(), 1);
    }

    #[test]
    fn test_delete_artifact() {
        let mut store = create_test_store().0;

        let reference = store
            .save_artifact(
                "session_1",
                ArtifactType::Pptx,
                "deck.pptx".to_string(),
                b"PK\x03\x04test".to_vec(),
                ArtifactMetadata::default(),
            )
            .unwrap();

        let id = reference.id.clone();
        store.delete_artifact("session_1", &id).unwrap();

        assert!(store.get_artifact_ref(&id).is_none());
        assert_eq!(store.session_count("session_1"), 0);
    }

    #[test]
    fn test_invalid_magic_bytes() {
        let mut store = create_test_store().0;

        let result = store.save_artifact(
            "session_1",
            ArtifactType::Pdf,
            "fake.pdf".to_string(),
            b"NOTPDF".to_vec(),
            ArtifactMetadata::default(),
        );

        assert!(matches!(result, Err(ArtifactError::InvalidData)));
    }

    #[test]
    fn test_size_limit() {
        let mut store = create_test_store().0;

        // Try to save a file larger than 50MB
        let large_content = vec![0u8; 51 * 1024 * 1024];

        let result = store.save_artifact(
            "session_1",
            ArtifactType::Docx,
            "large.docx".to_string(),
            large_content,
            ArtifactMetadata::default(),
        );

        assert!(matches!(
            result,
            Err(ArtifactError::SizeLimitExceeded { .. })
        ));
    }

    #[test]
    fn test_artifact_type_extensions() {
        assert_eq!(ArtifactType::Docx.extension(), ".docx");
        assert_eq!(ArtifactType::Pptx.extension(), ".pptx");
        assert_eq!(ArtifactType::Pdf.extension(), ".pdf");
    }

    #[test]
    fn test_artifact_type_mime_types() {
        assert!(ArtifactType::Docx.mime_type().contains("wordprocessingml"));
        assert!(ArtifactType::Pptx.mime_type().contains("presentationml"));
        assert_eq!(ArtifactType::Pdf.mime_type(), "application/pdf");
    }
}
