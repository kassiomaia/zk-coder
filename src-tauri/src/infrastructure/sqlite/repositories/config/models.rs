//! Model CRUD operations for the config store.

use rusqlite::{params, Connection, OptionalExtension};

use super::helpers::parse_datetime;
use super::ConfigRepositoryError;
use crate::core::config::{Model, ModelCapabilities};

/// Gets all models for a provider.
pub fn get_models(
    conn: &Connection,
    provider_id: &str,
) -> Result<Vec<Model>, ConfigRepositoryError> {
    let mut stmt = conn.prepare(
        "SELECT id, provider_id, name, supports_analysis, supports_code,
                supports_vision, supports_audio, supports_tips, supports_review, last_fetched
         FROM models WHERE provider_id = ?1 ORDER BY name",
    )?;

    let models = stmt
        .query_map(params![provider_id], |row| {
            let last_fetched_str: String = row.get(9)?;
            Ok(Model {
                id: row.get(0)?,
                provider_id: row.get(1)?,
                name: row.get(2)?,
                capabilities: ModelCapabilities {
                    analysis: row.get(3)?,
                    code: row.get(4)?,
                    vision: row.get(5)?,
                    audio: row.get(6)?,
                    tips: row.get(7)?,
                    review: row.get(8)?,
                },
                last_fetched: parse_datetime(&last_fetched_str),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(models)
}

/// Saves a single model to the database.
/// Note: This is for internal use; for replacing all models for a provider,
/// use the transaction-based method in ConfigStore::save_models.
pub fn save_single_model(conn: &Connection, model: &Model) -> Result<(), ConfigRepositoryError> {
    conn.execute(
        "INSERT OR REPLACE INTO models (id, provider_id, name, supports_analysis, supports_code,
         supports_vision, supports_audio, supports_tips, supports_review, last_fetched)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            &model.id,
            &model.provider_id,
            &model.name,
            model.capabilities.analysis,
            model.capabilities.code,
            model.capabilities.vision,
            model.capabilities.audio,
            model.capabilities.tips,
            model.capabilities.review,
            &model.last_fetched.to_rfc3339(),
        ],
    )?;

    Ok(())
}

/// Deletes all models for a provider.
pub fn delete_provider_models(
    conn: &Connection,
    provider_id: &str,
) -> Result<(), ConfigRepositoryError> {
    conn.execute(
        "DELETE FROM models WHERE provider_id = ?1",
        params![provider_id],
    )?;
    Ok(())
}

/// Saves models for a provider (replaces existing).
/// Note: This is a helper for tests. For production code, use the
/// transaction-based ConfigStore::save_models method.
pub fn save_models(
    conn: &Connection,
    provider_id: &str,
    models: &[Model],
) -> Result<(), ConfigRepositoryError> {
    delete_provider_models(conn, provider_id)?;
    for model in models {
        save_single_model(conn, model)?;
    }
    Ok(())
}

/// Adds a single model to the models table (without replacing existing models).
pub fn add_model(conn: &Connection, model: &Model) -> Result<(), ConfigRepositoryError> {
    conn.execute(
        "INSERT OR REPLACE INTO models (id, provider_id, name, supports_analysis, supports_code,
         supports_vision, supports_audio, supports_tips, supports_review, last_fetched)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            &model.id,
            &model.provider_id,
            &model.name,
            model.capabilities.analysis,
            model.capabilities.code,
            model.capabilities.vision,
            model.capabilities.audio,
            model.capabilities.tips,
            model.capabilities.review,
            &model.last_fetched.to_rfc3339(),
        ],
    )?;

    Ok(())
}

/// Removes a model from the models table by provider ID and model name.
pub fn delete_model_by_name(
    conn: &Connection,
    provider_id: &str,
    model_name: &str,
) -> Result<(), ConfigRepositoryError> {
    conn.execute(
        "DELETE FROM models WHERE provider_id = ?1 AND name = ?2",
        params![provider_id, model_name],
    )?;

    Ok(())
}

/// Gets a model by ID.
pub fn get_model(conn: &Connection, id: &str) -> Result<Option<Model>, ConfigRepositoryError> {
    let mut stmt = conn.prepare(
        "SELECT id, provider_id, name, supports_analysis, supports_code,
                supports_vision, supports_audio, supports_tips, supports_review, last_fetched
         FROM models WHERE id = ?1",
    )?;

    let model = stmt
        .query_row(params![id], |row| {
            let last_fetched_str: String = row.get(9)?;
            Ok(Model {
                id: row.get(0)?,
                provider_id: row.get(1)?,
                name: row.get(2)?,
                capabilities: ModelCapabilities {
                    analysis: row.get(3)?,
                    code: row.get(4)?,
                    vision: row.get(5)?,
                    audio: row.get(6)?,
                    tips: row.get(7)?,
                    review: row.get(8)?,
                },
                last_fetched: parse_datetime(&last_fetched_str),
            })
        })
        .optional()?;

    Ok(model)
}

/// Gets a model by provider ID and model name.
pub fn get_model_by_name(
    conn: &Connection,
    provider_id: &str,
    name: &str,
) -> Result<Option<Model>, ConfigRepositoryError> {
    let mut stmt = conn.prepare(
        "SELECT id, provider_id, name, supports_analysis, supports_code,
                supports_vision, supports_audio, supports_tips, supports_review, last_fetched
         FROM models WHERE provider_id = ?1 AND name = ?2",
    )?;

    let model = stmt
        .query_row(params![provider_id, name], |row| {
            let last_fetched_str: String = row.get(9)?;
            Ok(Model {
                id: row.get(0)?,
                provider_id: row.get(1)?,
                name: row.get(2)?,
                capabilities: ModelCapabilities {
                    analysis: row.get(3)?,
                    code: row.get(4)?,
                    vision: row.get(5)?,
                    audio: row.get(6)?,
                    tips: row.get(7)?,
                    review: row.get(8)?,
                },
                last_fetched: parse_datetime(&last_fetched_str),
            })
        })
        .optional()?;

    Ok(model)
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
    fn test_save_and_get_models() {
        let conn = make_test_store();
        let models = vec![
            Model::new("openai", "gpt-4", ModelCapabilities::default()),
            Model::new("openai", "gpt-3.5-turbo", ModelCapabilities::default()),
        ];

        save_models(&conn, "openai", &models).unwrap();

        let retrieved = get_models(&conn, "openai").unwrap();
        assert_eq!(retrieved.len(), 2);
    }

    #[test]
    fn test_add_model() {
        let conn = make_test_store();
        let model = Model::new("openai", "gpt-4", ModelCapabilities::default());
        add_model(&conn, &model).unwrap();

        let retrieved = get_models(&conn, "openai").unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].name, "gpt-4");
    }

    #[test]
    fn test_delete_model_by_name() {
        let conn = make_test_store();
        let model = Model::new("openai", "gpt-4", ModelCapabilities::default());
        add_model(&conn, &model).unwrap();

        delete_model_by_name(&conn, "openai", "gpt-4").unwrap();

        let retrieved = get_models(&conn, "openai").unwrap();
        assert_eq!(retrieved.len(), 0);
    }

    #[test]
    fn test_get_model() {
        let conn = make_test_store();
        let model = Model::new("openai", "gpt-4", ModelCapabilities::default());
        add_model(&conn, &model).unwrap();

        let retrieved = get_model(&conn, &model.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "gpt-4");
    }

    #[test]
    fn test_get_model_by_name() {
        let conn = make_test_store();
        let model = Model::new("openai", "gpt-4", ModelCapabilities::default());
        add_model(&conn, &model).unwrap();

        let retrieved = get_model_by_name(&conn, "openai", "gpt-4").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "gpt-4");
    }

    #[test]
    fn test_save_models_replaces_existing() {
        let conn = make_test_store();
        let models1 = vec![Model::new("openai", "gpt-4", ModelCapabilities::default())];
        save_models(&conn, "openai", &models1).unwrap();

        let models2 = vec![Model::new(
            "openai",
            "gpt-3.5-turbo",
            ModelCapabilities::default(),
        )];
        save_models(&conn, "openai", &models2).unwrap();

        let retrieved = get_models(&conn, "openai").unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].name, "gpt-3.5-turbo");
    }
}
