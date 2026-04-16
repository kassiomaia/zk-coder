//! Operation defaults operations for the config store.

use rusqlite::{params, Connection};

use super::ConfigRepositoryError;
use crate::core::config::{OperationDefault, OperationType};

/// Gets all operation defaults.
pub fn get_operation_defaults(
    conn: &Connection,
) -> Result<Vec<OperationDefault>, ConfigRepositoryError> {
    let mut stmt = conn.prepare("SELECT operation, model_id FROM operation_defaults")?;

    let defaults: Vec<OperationDefault> = stmt
        .query_map([], |row| {
            let operation_str: String = row.get(0)?;
            let operation: OperationType = operation_str
                .parse()
                .unwrap_or(OperationType::Analysis);

            // Read model_id as Option<String> to handle NULL values
            let model_id: Option<String> = row.get(1)?;

            Ok(OperationDefault {
                operation,
                model_id,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?
        // Filter out defaults with NULL model_id (orphaned entries)
        .into_iter()
        .filter(|d| d.model_id.is_some())
        .collect();

    // Clean up orphaned entries (where model_id is NULL)
    let _ = conn.execute("DELETE FROM operation_defaults WHERE model_id IS NULL", []);

    Ok(defaults)
}

/// Sets the default model for an operation.
pub fn set_operation_default(
    conn: &Connection,
    operation: OperationType,
    model_id: &str,
) -> Result<(), ConfigRepositoryError> {
    conn.execute(
        "INSERT OR REPLACE INTO operation_defaults (operation, model_id) VALUES (?1, ?2)",
        params![operation.as_str(), model_id],
    )?;

    Ok(())
}

/// Clears the default model for an operation.
pub fn clear_operation_default(
    conn: &Connection,
    operation: OperationType,
) -> Result<(), ConfigRepositoryError> {
    conn.execute(
        "DELETE FROM operation_defaults WHERE operation = ?1",
        params![operation.as_str()],
    )?;

    Ok(())
}

/// Clears all operation defaults for models belonging to a provider.
pub fn clear_defaults_for_provider(
    conn: &Connection,
    provider_id: &str,
) -> Result<(), ConfigRepositoryError> {
    conn.execute(
        "DELETE FROM operation_defaults WHERE model_id IN (SELECT id FROM models WHERE provider_id = ?1)",
        params![provider_id],
    )?;

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
    fn test_set_and_get_operation_defaults() {
        let conn = make_test_store();

        // First save a model (required for foreign key constraint)
        let model = crate::core::config::Model::new(
            "openai",
            "gpt-4",
            crate::core::config::ModelCapabilities::default(),
        );
        super::super::models::save_models(&conn, "openai", &[model]).unwrap();

        // Now set the operation default
        set_operation_default(&conn, OperationType::Code, "openai-gpt-4").unwrap();

        let defaults = get_operation_defaults(&conn).unwrap();
        assert_eq!(defaults.len(), 1);
        assert_eq!(defaults[0].operation, OperationType::Code);
        assert_eq!(defaults[0].model_id, Some("openai-gpt-4".to_string()));
    }

    #[test]
    fn test_clear_operation_default() {
        let conn = make_test_store();

        // Save a model first
        let model = crate::core::config::Model::new(
            "openai",
            "gpt-4",
            crate::core::config::ModelCapabilities::default(),
        );
        super::super::models::save_models(&conn, "openai", &[model]).unwrap();

        set_operation_default(&conn, OperationType::Code, "openai-gpt-4").unwrap();
        clear_operation_default(&conn, OperationType::Code).unwrap();

        let defaults = get_operation_defaults(&conn).unwrap();
        assert_eq!(defaults.len(), 0);
    }

    #[test]
    fn test_clear_defaults_for_provider() {
        let conn = make_test_store();

        // Save models for two providers
        let model1 = crate::core::config::Model::new(
            "openai",
            "gpt-4",
            crate::core::config::ModelCapabilities::default(),
        );
        let model2 = crate::core::config::Model::new(
            "anthropic",
            "claude-3",
            crate::core::config::ModelCapabilities::default(),
        );
        super::super::models::save_models(&conn, "openai", &[model1]).unwrap();
        super::super::models::save_models(&conn, "anthropic", &[model2]).unwrap();

        // Set defaults for both
        set_operation_default(&conn, OperationType::Code, "openai-gpt-4").unwrap();
        set_operation_default(&conn, OperationType::Analysis, "anthropic-claude-3").unwrap();

        // Clear defaults for openai
        clear_defaults_for_provider(&conn, "openai").unwrap();

        let defaults = get_operation_defaults(&conn).unwrap();
        assert_eq!(defaults.len(), 1);
        assert_eq!(defaults[0].model_id, Some("anthropic-claude-3".to_string()));
    }
}
