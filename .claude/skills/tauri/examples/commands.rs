// Example Tauri Commands

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::Mutex;
use std::sync::Arc;

// ============ State Definition ============

#[derive(Debug, Default)]
pub struct AppState {
    pub projects: Arc<Mutex<Vec<Project>>>,
    pub settings: Arc<Mutex<Settings>>,
}

// ============ Data Types ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: String,
    pub auto_save: bool,
    pub editor_font: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            auto_save: true,
            editor_font: "JetBrains Mono".to_string(),
        }
    }
}

// ============ Project Commands ============

/// Fetch all projects
#[tauri::command]
pub async fn fetch_projects(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    let projects = state.projects.lock().await.clone();
    Ok(projects)
}

/// Create a new project
#[tauri::command]
pub async fn create_project(
    state: State<'_, AppState>,
    name: String,
    path: String,
) -> Result<Project, String> {
    // Validation
    if name.trim().is_empty() {
        return Err("Project name cannot be empty".to_string());
    }

    if !std::path::Path::new(&path).exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    let project = Project {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.trim().to_string(),
        path,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let mut projects = state.projects.lock().await;
    projects.push(project.clone());

    Ok(project)
}

/// Delete a project by ID
#[tauri::command]
pub async fn delete_project(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut projects = state.projects.lock().await;
    let initial_len = projects.len();

    projects.retain(|p| p.id != id);

    if projects.len() == initial_len {
        return Err(format!("Project not found: {}", id));
    }

    Ok(())
}

// ============ Settings Commands ============

/// Get current settings
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let settings = state.settings.lock().await.clone();
    Ok(settings)
}

/// Update settings
#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<Settings, String> {
    let mut current = state.settings.lock().await;
    *current = settings.clone();
    Ok(settings)
}

// ============ File Operations ============

/// Read a file's contents
#[tauri::command]
pub async fn read_file(path: String) -> Result<String, String> {
    tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read file '{}': {}", path, e))
}

/// Write content to a file
#[tauri::command]
pub async fn write_file(path: String, content: String) -> Result<(), String> {
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&path).parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    tokio::fs::write(&path, content)
        .await
        .map_err(|e| format!("Failed to write file '{}': {}", path, e))
}

// ============ Command Registration ============
// In lib.rs or main.rs:
//
// tauri::Builder::default()
//     .manage(AppState::default())
//     .invoke_handler(tauri::generate_handler![
//         fetch_projects,
//         create_project,
//         delete_project,
//         get_settings,
//         update_settings,
//         read_file,
//         write_file,
//     ])
//     .run(tauri::generate_context!())
//     .expect("error while running tauri application");
