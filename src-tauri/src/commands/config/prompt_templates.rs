//! Tauri IPC commands for prompt templates.
//!
//! Thin adapter over `ConfigRepository`. The slug validation and the
//! builtin-vs-custom invariants are enforced by the repository layer; this
//! module maps request payloads to the domain type and domain errors to
//! user-facing strings.

use chrono::Utc;

use super::state::ConfigState;
use crate::core::config::{PromptTemplate, SavePromptTemplateRequest};

/// Lists every prompt template ordered by sort order then slug.
#[tauri::command]
pub async fn list_prompt_templates(
    state: tauri::State<'_, ConfigState>,
) -> Result<Vec<PromptTemplate>, String> {
    state
        .repository
        .list_prompt_templates()
        .map_err(|e| e.to_string())
}

/// Creates a new prompt template or updates an existing one.
#[tauri::command]
pub async fn save_prompt_template(
    request: SavePromptTemplateRequest,
    state: tauri::State<'_, ConfigState>,
) -> Result<PromptTemplate, String> {
    let slug = request.slug.trim().to_string();
    if !PromptTemplate::is_valid_slug(&slug) {
        return Err(format!(
            "Invalid slug '{slug}'. Use lowercase letters, digits, and dashes; must start with a letter or digit."
        ));
    }
    if request.title.trim().is_empty() {
        return Err("Title must not be empty".to_string());
    }
    if request.content.trim().is_empty() {
        return Err("Content must not be empty".to_string());
    }
    if request.category.trim().is_empty() {
        return Err("Category must not be empty".to_string());
    }

    let repo = &state.repository;
    let now = Utc::now();

    let template = if let Some(id) = request.id.as_deref() {
        // Update path: load existing to preserve identity fields.
        let existing = repo
            .list_prompt_templates()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|t| t.id == id)
            .ok_or_else(|| format!("Prompt template not found: {id}"))?;

        // Builtins keep their slug — it's the stable handle the resolver uses.
        let effective_slug = if existing.is_builtin {
            existing.slug.clone()
        } else {
            slug.clone()
        };

        // Reject slug collision with some *other* template.
        if let Some(collision) = repo
            .get_prompt_template_by_slug(&effective_slug)
            .map_err(|e| e.to_string())?
        {
            if collision.id != existing.id {
                return Err(format!("A template with slug '{effective_slug}' already exists"));
            }
        }

        PromptTemplate {
            id: existing.id,
            slug: effective_slug,
            title: request.title,
            description: request.description,
            icon: request.icon,
            category: request.category,
            content: request.content,
            is_builtin: existing.is_builtin,
            is_enabled: request.is_enabled,
            sort_order: existing.sort_order,
            created_at: existing.created_at,
            updated_at: now,
        }
    } else {
        // Create path.
        if repo
            .get_prompt_template_by_slug(&slug)
            .map_err(|e| e.to_string())?
            .is_some()
        {
            return Err(format!("A template with slug '{slug}' already exists"));
        }
        PromptTemplate {
            id: uuid::Uuid::new_v4().to_string(),
            slug,
            title: request.title,
            description: request.description,
            icon: request.icon,
            category: request.category,
            content: request.content,
            is_builtin: false,
            is_enabled: request.is_enabled,
            // User templates sort after builtins by default.
            sort_order: 1000,
            created_at: now,
            updated_at: now,
        }
    };

    repo.upsert_prompt_template(&template)
        .map_err(|e| e.to_string())
}

/// Deletes a prompt template. Builtins are rejected by the repository.
#[tauri::command]
pub async fn delete_prompt_template(
    id: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    state
        .repository
        .delete_prompt_template(&id)
        .map_err(|e| e.to_string())
}

/// Resets a builtin prompt template to its seeded default.
#[tauri::command]
pub async fn reset_prompt_template(
    slug: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<PromptTemplate, String> {
    state
        .repository
        .reset_prompt_template(&slug)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::config::state::ConfigState;

    fn make_state() -> ConfigState {
        ConfigState::in_memory().expect("in-memory config state")
    }

    fn save_request(slug: &str) -> SavePromptTemplateRequest {
        SavePromptTemplateRequest {
            id: None,
            slug: slug.to_string(),
            title: "Custom".to_string(),
            description: "desc".to_string(),
            icon: None,
            category: "CUSTOM".to_string(),
            content: "hello {args}".to_string(),
            is_enabled: true,
        }
    }

    #[tokio::test]
    async fn list_returns_seeded_builtins() {
        let state = make_state();
        let templates = list_prompt_templates(crate::test_support::state(&state))
            .await
            .unwrap();
        assert!(templates.iter().any(|t| t.slug == "refactor"));
        assert!(templates.iter().any(|t| t.slug == "generate"));
    }

    #[tokio::test]
    async fn save_rejects_invalid_slug() {
        let state = make_state();
        let req = save_request("Bad Slug");
        let err = save_prompt_template(req, crate::test_support::state(&state))
            .await
            .unwrap_err();
        assert!(err.contains("Invalid slug"));
    }

    #[tokio::test]
    async fn save_rejects_duplicate_slug() {
        let state = make_state();
        let req = save_request("refactor");
        let err = save_prompt_template(req, crate::test_support::state(&state))
            .await
            .unwrap_err();
        assert!(err.contains("already exists"));
    }

    #[tokio::test]
    async fn save_creates_and_updates_custom_template() {
        let state = make_state();
        let created = save_prompt_template(save_request("note"), crate::test_support::state(&state))
            .await
            .unwrap();
        assert!(!created.is_builtin);

        let mut update = save_request("note");
        update.id = Some(created.id.clone());
        update.title = "Updated".to_string();
        let updated = save_prompt_template(update, crate::test_support::state(&state))
            .await
            .unwrap();
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.title, "Updated");
    }

    #[tokio::test]
    async fn delete_rejects_builtins_via_command() {
        let state = make_state();
        let templates = list_prompt_templates(crate::test_support::state(&state))
            .await
            .unwrap();
        let refactor = templates.iter().find(|t| t.slug == "refactor").unwrap();
        let err = delete_prompt_template(refactor.id.clone(), crate::test_support::state(&state))
            .await
            .unwrap_err();
        assert!(err.contains("not found"));
    }

    #[tokio::test]
    async fn reset_restores_builtin_defaults() {
        let state = make_state();

        // Mutate refactor through the save command.
        let templates = list_prompt_templates(crate::test_support::state(&state))
            .await
            .unwrap();
        let refactor = templates.iter().find(|t| t.slug == "refactor").unwrap();
        let mut edit = save_request("refactor");
        edit.id = Some(refactor.id.clone());
        edit.content = "EDITED {args}".to_string();
        let edited = save_prompt_template(edit, crate::test_support::state(&state))
            .await
            .unwrap();
        assert_eq!(edited.content, "EDITED {args}");

        let reset = reset_prompt_template(
            "refactor".to_string(),
            crate::test_support::state(&state),
        )
        .await
        .unwrap();
        assert_ne!(reset.content, "EDITED {args}");
        assert!(reset.content.contains("Refactor the following code"));
    }
}
