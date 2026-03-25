//! Provider CRUD operations for the config store.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use super::ConfigStoreError;
use super::helpers::parse_datetime;
use crate::nlp::config::{ApiType, ModelsSource, Provider};

/// Gets all providers.
pub fn get_providers(conn: &Connection) -> Result<Vec<Provider>, ConfigStoreError> {
    log::info!("config_store::providers::get_providers: Preparing query...");

    let mut stmt = conn.prepare(
        "SELECT id, name, icon, is_custom, api_type, base_url, is_configured, models_source, created_at, updated_at
         FROM providers ORDER BY name",
    )?;
    log::info!("config_store::providers::get_providers: Query prepared, executing...");

    let providers = stmt
        .query_map([], |row| {
            let api_type_str: String = row.get(4)?;
            let api_type = ApiType::from_str(&api_type_str);
            let models_source_str: String = row.get(7)?;
            let models_source = ModelsSource::from_str(&models_source_str);
            let created_at_str: String = row.get(8)?;
            let updated_at_str: String = row.get(9)?;

            log::debug!(
                "config_store::providers::get_providers: Row id={}, name={}, is_configured={}",
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, bool>(6)?
            );

            Ok(Provider {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                is_custom: row.get(3)?,
                api_type,
                base_url: row.get(5)?,
                is_configured: row.get(6)?,
                models_source,
                created_at: parse_datetime(&created_at_str),
                updated_at: parse_datetime(&updated_at_str),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(providers)
}

/// Gets a provider by ID.
pub fn get_provider(conn: &Connection, id: &str) -> Result<Option<Provider>, ConfigStoreError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, icon, is_custom, api_type, base_url, is_configured, models_source, created_at, updated_at
         FROM providers WHERE id = ?1",
    )?;

    let provider = stmt
        .query_row(params![id], |row| {
            let api_type_str: String = row.get(4)?;
            let api_type = ApiType::from_str(&api_type_str);
            let models_source_str: String = row.get(7)?;
            let models_source = ModelsSource::from_str(&models_source_str);
            let created_at_str: String = row.get(8)?;
            let updated_at_str: String = row.get(9)?;

            Ok(Provider {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                is_custom: row.get(3)?,
                api_type,
                base_url: row.get(5)?,
                is_configured: row.get(6)?,
                models_source,
                created_at: parse_datetime(&created_at_str),
                updated_at: parse_datetime(&updated_at_str),
            })
        })
        .optional()?;

    Ok(provider)
}

/// Saves a provider (create or update).
pub fn save_provider(conn: &Connection, provider: &Provider) -> Result<(), ConfigStoreError> {
    conn.execute(
        "INSERT INTO providers (id, name, icon, is_custom, api_type, base_url, is_configured, models_source, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            icon = excluded.icon,
            api_type = excluded.api_type,
            base_url = excluded.base_url,
            is_configured = excluded.is_configured,
            models_source = excluded.models_source,
            updated_at = excluded.updated_at",
        params![
            &provider.id,
            &provider.name,
            &provider.icon,
            provider.is_custom,
            provider.api_type.as_str(),
            &provider.base_url,
            provider.is_configured,
            provider.models_source.as_str(),
            &provider.created_at.to_rfc3339(),
            &provider.updated_at.to_rfc3339(),
        ],
    )?;

    Ok(())
}

/// Deletes a provider.
pub fn delete_provider(conn: &Connection, id: &str) -> Result<(), ConfigStoreError> {
    let rows_affected = conn.execute(
        "DELETE FROM providers WHERE id = ?1 AND is_custom = TRUE",
        params![id],
    )?;

    if rows_affected == 0 {
        return Err(ConfigStoreError::NotFound("Provider", id.to_string()));
    }

    Ok(())
}

/// Sets the configured status of a provider.
pub fn set_provider_configured(
    conn: &Connection,
    id: &str,
    configured: bool,
) -> Result<(), ConfigStoreError> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE providers SET is_configured = ?1, updated_at = ?2 WHERE id = ?3",
        params![configured, now, id],
    )?;

    Ok(())
}

/// Sets the models source mode for a provider.
pub fn set_provider_models_source(
    conn: &Connection,
    provider_id: &str,
    models_source: ModelsSource,
) -> Result<(), ConfigStoreError> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE providers SET models_source = ?1, updated_at = ?2 WHERE id = ?3",
        params![models_source.as_str(), now, provider_id],
    )?;

    Ok(())
}

/// Seeds built-in providers into the database.
pub fn seed_builtin_providers(conn: &Connection) -> Result<(), ConfigStoreError> {
    let builtin_providers = [
        ("anthropic", "Anthropic", "AC", "anthropic"),
        ("openai", "OpenAI", "OA", "openai"),
        ("google", "Google", "GC", "google"),
    ];

    let now = Utc::now().to_rfc3339();
    for (id, name, icon, api_type) in builtin_providers {
        conn.execute(
            "INSERT OR IGNORE INTO providers (id, name, icon, is_custom, api_type, is_configured, models_source, created_at, updated_at)
             VALUES (?1, ?2, ?3, FALSE, ?4, FALSE, 'auto', ?5, ?5)",
            params![id, name, icon, api_type, now],
        )?;
    }

    // Ensure built-in providers always have correct values (fixes any corrupted data)
    conn.execute_batch(
        r#"
        UPDATE providers
        SET is_custom = FALSE, is_configured = FALSE, api_type =
            CASE
                WHEN id = 'anthropic' THEN 'anthropic'
                WHEN id = 'google' THEN 'google'
                WHEN id = 'openai' THEN 'openai'
                ELSE api_type
            END,
            models_source = 'auto'
        WHERE id IN ('anthropic', 'openai', 'google');
        "#,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::config_store::schema;

    fn make_test_store() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        schema::init_schema(&conn).unwrap();
        seed_builtin_providers(&conn).unwrap();
        conn
    }

    #[test]
    fn test_get_providers() {
        let conn = make_test_store();
        let providers = get_providers(&conn).unwrap();
        assert_eq!(providers.len(), 3);
    }

    #[test]
    fn test_save_and_get_provider() {
        let conn = make_test_store();
        let provider = Provider::custom(
            "Test Provider",
            "TP",
            ApiType::OpenAI,
            Some("https://api.test.com".to_string()),
        );
        save_provider(&conn, &provider).unwrap();

        let retrieved = get_provider(&conn, &provider.id).unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "Test Provider");
        assert!(retrieved.is_custom);
    }

    #[test]
    fn test_delete_provider() {
        let conn = make_test_store();
        let provider = Provider::custom("ToDelete", "TD", ApiType::OpenAI, None);
        save_provider(&conn, &provider).unwrap();

        delete_provider(&conn, &provider.id).unwrap();

        let retrieved = get_provider(&conn, &provider.id).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_delete_builtin_provider_fails() {
        let conn = make_test_store();
        let result = delete_provider(&conn, "openai");
        assert!(result.is_err());
    }

    #[test]
    fn test_set_provider_configured() {
        let conn = make_test_store();
        set_provider_configured(&conn, "openai", true).unwrap();

        let provider = get_provider(&conn, "openai").unwrap().unwrap();
        assert!(provider.is_configured);
    }

    #[test]
    fn test_set_provider_models_source() {
        let conn = make_test_store();
        set_provider_models_source(&conn, "openai", ModelsSource::Manual).unwrap();

        let provider = get_provider(&conn, "openai").unwrap().unwrap();
        assert_eq!(provider.models_source, ModelsSource::Manual);
    }

    #[test]
    fn test_get_provider_not_found() {
        let conn = make_test_store();
        let result = get_provider(&conn, "nonexistent");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
