//! Document artifact commands.

use crate::infrastructure::tools::documents::shared::{
    ArtifactMetadata, ArtifactRef, ArtifactStore, ArtifactType,
};
use std::sync::Mutex;
use tauri::State;

/// List artifacts for a session.
#[tauri::command]
pub fn list_artifacts(
    session_id: String,
    store: State<'_, Mutex<ArtifactStore>>,
) -> Result<Vec<ArtifactRef>, String> {
    let store = store.lock().map_err(|e| e.to_string())?;
    Ok(store.list_session_artifacts(&session_id))
}

/// List all artifacts across all sessions.
#[tauri::command]
pub fn list_all_artifacts(store: State<'_, Mutex<ArtifactStore>>) -> Result<Vec<ArtifactRef>, String> {
    let store = store.lock().map_err(|e| e.to_string())?;
    Ok(store.list_all_artifacts())
}

/// Get artifact content bytes for download.
#[tauri::command]
pub fn download_artifact(
    artifact_id: String,
    store: State<'_, Mutex<ArtifactStore>>,
) -> Result<(String, Vec<u8>), String> {
    let store = store.lock().map_err(|e| e.to_string())?;
    let reference = store
        .get_artifact_ref(&artifact_id)
        .ok_or_else(|| format!("Artifact not found: {}", artifact_id))?;

    let content = store.get_artifact_content(&artifact_id).map_err(|e| e.to_string())?;

    Ok((reference.filename.clone(), content))
}

/// Get artifact reference without content.
#[tauri::command]
pub fn get_artifact(
    artifact_id: String,
    store: State<'_, Mutex<ArtifactStore>>,
) -> Result<ArtifactRef, String> {
    let store = store.lock().map_err(|e| e.to_string())?;
    let reference = store
        .get_artifact_ref(&artifact_id)
        .ok_or_else(|| format!("Artifact not found: {}", artifact_id))?;
    Ok(reference.clone())
}

/// Arguments for saving an artifact.
#[derive(Debug, serde::Deserialize)]
pub struct SaveArtifactArgs {
    pub session_id: String,
    pub artifact_type: ArtifactType,
    pub filename: String,
    pub content: Vec<u8>,
    #[serde(default)]
    pub metadata: ArtifactMetadata,
}

/// Save a new artifact.
#[tauri::command]
pub fn save_artifact(
    args: SaveArtifactArgs,
    store: State<'_, Mutex<ArtifactStore>>,
) -> Result<ArtifactRef, String> {
    let mut store = store.lock().map_err(|e| e.to_string())?;

    store
        .save_artifact(
            &args.session_id,
            args.artifact_type,
            args.filename,
            args.content,
            args.metadata,
        )
        .map_err(|e| e.to_string())
}

/// Arguments for updating artifact metadata.
#[derive(Debug, serde::Deserialize)]
pub struct UpdateMetadataArgs {
    pub artifact_id: String,
    pub metadata: ArtifactMetadata,
}

/// Update artifact metadata.
#[tauri::command]
pub fn update_metadata(
    args: UpdateMetadataArgs,
    store: State<'_, Mutex<ArtifactStore>>,
) -> Result<ArtifactRef, String> {
    let mut store = store.lock().map_err(|e| e.to_string())?;
    store
        .update_metadata(&args.artifact_id, args.metadata)
        .map_err(|e| e.to_string())
}

/// Delete an artifact.
#[tauri::command]
pub fn delete_artifact(
    session_id: String,
    artifact_id: String,
    store: State<'_, Mutex<ArtifactStore>>,
) -> Result<(), String> {
    let mut store = store.lock().map_err(|e| e.to_string())?;
    store
        .delete_artifact(&session_id, &artifact_id)
        .map_err(|e| e.to_string())
}

/// Clean up all artifacts for a session.
#[tauri::command]
pub fn cleanup_session(
    session_id: String,
    store: State<'_, Mutex<ArtifactStore>>,
) -> Result<usize, String> {
    let mut store = store.lock().map_err(|e| e.to_string())?;
    store
        .cleanup_session(&session_id)
        .map_err(|e| e.to_string())
}

/// Get artifact count for a session.
#[tauri::command]
pub fn count_session(
    session_id: String,
    store: State<'_, Mutex<ArtifactStore>>,
) -> Result<usize, String> {
    let store = store.lock().map_err(|e| e.to_string())?;
    Ok(store.session_count(&session_id))
}
