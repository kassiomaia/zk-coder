//! Manual model operations for the config store.

use chrono::Utc;
use rusqlite::{params, Connection};

use super::ConfigRepositoryError;
use crate::core::config::{ManualModelEntry, ModelCapabilities};

/// Gets all manual models for a provider.
pub fn get_manual_models(
    conn: &Connection,
    provider_id: &str,
) -> Result<Vec<ManualModelEntry>, ConfigRepositoryError> {
    let mut stmt = conn.prepare(
        "SELECT name, supports_analysis, supports_code, supports_vision,
                supports_audio, supports_tips, supports_review
         FROM manual_models WHERE provider_id = ?1 ORDER BY name",
    )?;

    let models = stmt
        .query_map(params![provider_id], |row| {
            Ok(ManualModelEntry {
                name: row.get(0)?,
                capabilities: ModelCapabilities {
                    analysis: row.get(1)?,
                    code: row.get(2)?,
                    vision: row.get(3)?,
                    audio: row.get(4)?,
                    tips: row.get(5)?,
                    review: row.get(6)?,
                },
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(models)
}

/// Adds a manual model for a provider.
pub fn add_manual_model(
    conn: &Connection,
    provider_id: &str,
    model: &ManualModelEntry,
) -> Result<(), ConfigRepositoryError> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO manual_models (provider_id, name, supports_analysis, supports_code,
         supports_vision, supports_audio, supports_tips, supports_review, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            provider_id,
            &model.name,
            model.capabilities.analysis,
            model.capabilities.code,
            model.capabilities.vision,
            model.capabilities.audio,
            model.capabilities.tips,
            model.capabilities.review,
            now,
        ],
    )?;

    Ok(())
}

/// Removes a manual model for a provider.
pub fn remove_manual_model(
    conn: &Connection,
    provider_id: &str,
    model_name: &str,
) -> Result<(), ConfigRepositoryError> {
    let rows_affected = conn.execute(
        "DELETE FROM manual_models WHERE provider_id = ?1 AND name = ?2",
        params![provider_id, model_name],
    )?;

    if rows_affected == 0 {
        return Err(ConfigRepositoryError::NotFound(
            "Manual model",
            format!("{} for provider {}", model_name, provider_id),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_store() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::config_schema::init_schema(&conn).unwrap();
        super::super::providers::seed_builtin_providers(&conn).unwrap();
        conn
    }

    #[test]
    fn test_add_and_get_manual_models() {
        let conn = make_test_store();

        let model = ManualModelEntry {
            name: "custom-model".to_string(),
            capabilities: ModelCapabilities::default(),
        };

        add_manual_model(&conn, "openai", &model).unwrap();

        let models = get_manual_models(&conn, "openai").unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "custom-model");
    }

    #[test]
    fn test_remove_manual_model() {
        let conn = make_test_store();

        let model = ManualModelEntry {
            name: "custom-model".to_string(),
            capabilities: ModelCapabilities::default(),
        };

        add_manual_model(&conn, "openai", &model).unwrap();
        remove_manual_model(&conn, "openai", "custom-model").unwrap();

        let models = get_manual_models(&conn, "openai").unwrap();
        assert_eq!(models.len(), 0);
    }

    #[test]
    fn test_remove_nonexistent_manual_model() {
        let conn = make_test_store();
        let result = remove_manual_model(&conn, "openai", "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_manual_models_isolated_by_provider() {
        let conn = make_test_store();

        let model = ManualModelEntry {
            name: "custom-model".to_string(),
            capabilities: ModelCapabilities::default(),
        };

        add_manual_model(&conn, "openai", &model).unwrap();

        let openai_models = get_manual_models(&conn, "openai").unwrap();
        let anthropic_models = get_manual_models(&conn, "anthropic").unwrap();

        assert_eq!(openai_models.len(), 1);
        assert_eq!(anthropic_models.len(), 0);
    }
}
